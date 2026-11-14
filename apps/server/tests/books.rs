//! Upload pipeline: stream -> content-addressed blob -> books row, plus the
//! read paths that make the row useful (list, single, ranged file).

use axum::body::Body;
use axum::http::{HeaderMap, Request, StatusCode, header};
use http_body_util::BodyExt;
use leaflet::config::Config;
use leaflet::{AppState, router};
use serde_json::json;
use tower::ServiceExt;
use uuid::Uuid;

const PDF: &[u8] = b"%PDF-1.7\n1 0 obj\n<< /Type /Catalog >>\nendobj\ntrailer\n%%EOF\n";

fn epub() -> Vec<u8> {
    let mut bytes =
        b"PK\x03\x04\x14\x00\x00\x00\x08\x00\x1b\x00mimetypeapplication/epub+zip".to_vec();
    bytes.extend_from_slice(&[0u8; 64]);
    bytes
}

async fn state(max_upload_bytes: u64) -> (AppState, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("leaflet-books-api-{}", Uuid::now_v7()));
    let config = Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        database_url: format!("sqlite://{}?mode=rwc", dir.join("leaflet.db").display()),
        data_dir: dir.clone(),
        web_dir: dir.join("web"),
        invite_code: Some("invite-me".into()),
        cookie_secure: false,
        log: "error".into(),
        max_upload_bytes,
    };
    let pool = leaflet::db::open(&config).await.unwrap();
    let storage = leaflet::storage::Storage::new(config.blobs_dir()).unwrap();
    let assets = leaflet::assets::Assets::new(&config.web_dir);
    let signing = leaflet::auth::tokens::SigningKey::from_bytes(&[7u8; 64]).unwrap();
    (
        AppState {
            config,
            pool,
            storage,
            assets,
            signing,
        },
        dir,
    )
}

struct Reply {
    status: StatusCode,
    headers: HeaderMap,
    body: Vec<u8>,
    json: serde_json::Value,
}

impl Reply {
    fn header(&self, name: header::HeaderName) -> &str {
        self.headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
    }
}

async fn send(state: &AppState, request: Request<Body>) -> Reply {
    let response = router(state.clone()).oneshot(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes()
        .to_vec();
    let json = serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
    Reply {
        status,
        headers,
        body,
        json,
    }
}

async fn register(state: &AppState, email: &str) -> String {
    let reply = send(
        state,
        Request::post("/api/v1/auth/register")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                // the bootstrap account ignores the code, later ones need it
                json!({ "email": email, "display_name": "Reader",
                        "password": "correct horse battery", "invite_code": "invite-me" })
                .to_string(),
            ))
            .unwrap(),
    )
    .await;
    assert_eq!(
        reply.status,
        StatusCode::CREATED,
        "register: {}",
        reply.json
    );
    reply.json["access_token"].as_str().unwrap().to_owned()
}

fn multipart(filename: &str, bytes: &[u8]) -> (String, Vec<u8>) {
    let boundary = "leaflet-test-boundary";
    let mut body = format!(
        "--{boundary}\r\ncontent-disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\n\
         content-type: application/octet-stream\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    (format!("multipart/form-data; boundary={boundary}"), body)
}

async fn upload(state: &AppState, token: &str, filename: &str, bytes: &[u8]) -> Reply {
    let (content_type, body) = multipart(filename, bytes);
    send(
        state,
        Request::post("/api/v1/books")
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header(header::CONTENT_TYPE, content_type)
            .body(Body::from(body))
            .unwrap(),
    )
    .await
}

#[tokio::test]
async fn uploading_a_pdf_stores_the_bytes_and_returns_the_book() {
    let (state, dir) = state(1024 * 1024).await;
    let token = register(&state, "reader@example.com").await;

    let reply = upload(&state, &token, "Dune.pdf", PDF).await;

    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.json);
    assert_eq!(
        reply.json["title"], "Dune",
        "title falls back to the filename"
    );
    assert_eq!(reply.json["format"], "pdf");
    assert_eq!(reply.json["byte_size"], PDF.len());
    assert_eq!(
        reply.json["sha256"],
        leaflet::storage::sha256_hex(PDF).as_str()
    );
    assert_eq!(
        reply.header(header::LOCATION),
        format!("/api/v1/books/{}", reply.json["id"].as_str().unwrap())
    );

    let sha = leaflet::storage::sha256_hex(PDF);
    assert!(
        state.storage.contains(&sha).await,
        "blob is on disk under its hash"
    );
    assert_eq!(
        tokio::fs::read(state.storage.path_for(&sha)).await.unwrap(),
        PDF
    );

    std::fs::remove_dir_all(dir).ok();
}

#[tokio::test]
async fn uploading_the_same_file_twice_keeps_one_book() {
    let (state, dir) = state(1024 * 1024).await;
    let token = register(&state, "reader@example.com").await;

    let first = upload(&state, &token, "Dune.pdf", PDF).await;
    let again = upload(&state, &token, "dune (copy).pdf", PDF).await;

    assert_eq!(first.status, StatusCode::CREATED);
    assert_eq!(again.status, StatusCode::OK, "re-upload is not an error");
    assert_eq!(again.json["id"], first.json["id"]);
    assert_eq!(again.json["title"], "Dune", "the original title wins");

    let (rows,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM books")
        .fetch_one(&state.pool)
        .await
        .unwrap();
    assert_eq!(rows, 1);

    std::fs::remove_dir_all(dir).ok();
}

#[tokio::test]
async fn uploading_an_epub_is_accepted() {
    let (state, dir) = state(1024 * 1024).await;
    let token = register(&state, "reader@example.com").await;

    let reply = upload(&state, &token, "piranesi.epub", &epub()).await;

    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.json);
    assert_eq!(reply.json["format"], "epub");
    assert_eq!(reply.json["title"], "piranesi");

    std::fs::remove_dir_all(dir).ok();
}

#[tokio::test]
async fn files_that_are_not_books_are_rejected_and_leave_nothing_behind() {
    let (state, dir) = state(1024 * 1024).await;
    let token = register(&state, "reader@example.com").await;

    let reply = upload(&state, &token, "notes.txt", b"just some text\n").await;

    assert_eq!(
        reply.status,
        StatusCode::UNSUPPORTED_MEDIA_TYPE,
        "{}",
        reply.json
    );
    assert_eq!(reply.json["type"], "unsupported_format");

    let sha = leaflet::storage::sha256_hex(b"just some text\n");
    assert!(
        !state.storage.contains(&sha).await,
        "a rejected upload is not kept"
    );
    let tmp = std::fs::read_dir(state.storage.root().join(".tmp")).unwrap();
    assert_eq!(tmp.count(), 0, "no temp files survive a rejection");

    std::fs::remove_dir_all(dir).ok();
}

#[tokio::test]
async fn an_empty_upload_is_a_client_error() {
    let (state, dir) = state(1024 * 1024).await;
    let token = register(&state, "reader@example.com").await;

    let reply = upload(&state, &token, "empty.pdf", b"").await;

    assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{}", reply.json);
    assert_eq!(reply.json["type"], "invalid_request");

    std::fs::remove_dir_all(dir).ok();
}

#[tokio::test]
async fn an_oversized_upload_is_refused() {
    let (state, dir) = state(64).await;
    let token = register(&state, "reader@example.com").await;

    let big = [PDF, &[b'x'; 128][..]].concat();
    let reply = upload(&state, &token, "Dune.pdf", &big).await;

    assert_eq!(
        reply.status,
        StatusCode::PAYLOAD_TOO_LARGE,
        "{}",
        reply.json
    );
    assert_eq!(reply.json["type"], "payload_too_large");
    assert!(
        !state
            .storage
            .contains(&leaflet::storage::sha256_hex(&big))
            .await
    );
    assert_eq!(
        std::fs::read_dir(state.storage.root().join(".tmp"))
            .unwrap()
            .count(),
        0
    );

    std::fs::remove_dir_all(dir).ok();
}

#[tokio::test]
async fn a_multipart_body_without_a_file_field_is_rejected() {
    let (state, dir) = state(1024 * 1024).await;
    let token = register(&state, "reader@example.com").await;

    let body = "--b\r\ncontent-disposition: form-data; name=\"title\"\r\n\r\nDune\r\n--b--\r\n";
    let reply = send(
        &state,
        Request::post("/api/v1/books")
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header(header::CONTENT_TYPE, "multipart/form-data; boundary=b")
            .body(Body::from(body))
            .unwrap(),
    )
    .await;

    assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{}", reply.json);
    assert_eq!(reply.json["type"], "invalid_request");

    std::fs::remove_dir_all(dir).ok();
}

#[tokio::test]
async fn uploading_requires_an_access_token() {
    let (state, dir) = state(1024 * 1024).await;
    let (content_type, body) = multipart("Dune.pdf", PDF);

    let reply = send(
        &state,
        Request::post("/api/v1/books")
            .header(header::CONTENT_TYPE, content_type)
            .body(Body::from(body))
            .unwrap(),
    )
    .await;

    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    std::fs::remove_dir_all(dir).ok();
}

#[tokio::test]
async fn a_library_is_private_to_its_owner() {
    let (state, dir) = state(1024 * 1024).await;
    let reader = register(&state, "reader@example.com").await;
    let book = upload(&state, &reader, "Dune.pdf", PDF).await.json;
    let id = book["id"].as_str().unwrap().to_owned();

    let intruder = register(&state, "intruder@example.com").await;
    let shelf = send(
        &state,
        Request::get("/api/v1/books")
            .header(header::AUTHORIZATION, format!("Bearer {intruder}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(shelf.status, StatusCode::OK);
    assert_eq!(shelf.json, json!([]), "another member's shelf is empty");

    let peek = send(
        &state,
        Request::get(format!("/api/v1/books/{id}"))
            .header(header::AUTHORIZATION, format!("Bearer {intruder}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(
        peek.status,
        StatusCode::NOT_FOUND,
        "not 403: don't confirm it exists"
    );

    let steal = send(
        &state,
        Request::get(format!("/api/v1/books/{id}/file"))
            .header(header::AUTHORIZATION, format!("Bearer {intruder}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(steal.status, StatusCode::NOT_FOUND);

    std::fs::remove_dir_all(dir).ok();
}

#[tokio::test]
async fn the_file_endpoint_serves_whole_files_and_ranges() {
    let (state, dir) = state(1024 * 1024).await;
    let token = register(&state, "reader@example.com").await;
    let id = upload(&state, &token, "Dune.pdf", PDF).await.json["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let path = format!("/api/v1/books/{id}/file");

    let whole = send(
        &state,
        Request::get(&path)
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(whole.status, StatusCode::OK);
    assert_eq!(whole.body, PDF);
    assert_eq!(whole.header(header::CONTENT_TYPE), "application/pdf");
    assert_eq!(whole.header(header::ACCEPT_RANGES), "bytes");
    assert!(
        whole
            .header(header::CONTENT_DISPOSITION)
            .contains("filename=\"Dune.pdf\"")
    );

    let ranged = send(
        &state,
        Request::get(&path)
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header(header::RANGE, "bytes=0-3")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(ranged.status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(ranged.body, b"%PDF");
    assert_eq!(
        ranged.header(header::CONTENT_RANGE),
        format!("bytes 0-3/{}", PDF.len())
    );
    assert_eq!(ranged.header(header::CONTENT_LENGTH), "4");

    let suffix = send(
        &state,
        Request::get(&path)
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header(header::RANGE, "bytes=-6")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(suffix.status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(suffix.body, b"%%EOF\n");

    let beyond = send(
        &state,
        Request::get(&path)
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header(header::RANGE, "bytes=9999-")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(beyond.status, StatusCode::RANGE_NOT_SATISFIABLE);
    assert_eq!(
        beyond.header(header::CONTENT_RANGE),
        format!("bytes */{}", PDF.len())
    );

    std::fs::remove_dir_all(dir).ok();
}

#[tokio::test]
async fn repeat_reads_hit_the_etag() {
    let (state, dir) = state(1024 * 1024).await;
    let token = register(&state, "reader@example.com").await;
    let id = upload(&state, &token, "Dune.pdf", PDF).await.json["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let path = format!("/api/v1/books/{id}/file");

    let first = send(
        &state,
        Request::get(&path)
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    let etag = first.header(header::ETAG).to_owned();
    assert_eq!(etag, format!("\"{}\"", leaflet::storage::sha256_hex(PDF)));

    let cached = send(
        &state,
        Request::get(&path)
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header(header::IF_NONE_MATCH, &etag)
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(cached.status, StatusCode::NOT_MODIFIED);
    assert!(cached.body.is_empty());

    std::fs::remove_dir_all(dir).ok();
}

#[tokio::test]
async fn the_shelf_is_listed_alphabetically() {
    let (state, dir) = state(1024 * 1024).await;
    let token = register(&state, "reader@example.com").await;
    for (name, body) in [
        ("The Left Hand of Darkness.pdf", PDF),
        ("Ancillary Justice.pdf", b"%PDF-1.4 second".as_slice()),
    ] {
        let reply = upload(&state, &token, name, body).await;
        assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.json);
    }

    let shelf = send(
        &state,
        Request::get("/api/v1/books")
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(shelf.status, StatusCode::OK);
    let titles: Vec<_> = shelf
        .json
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["title"].clone())
        .collect();
    assert_eq!(
        titles,
        vec![
            json!("Ancillary Justice"),
            json!("The Left Hand of Darkness")
        ]
    );

    std::fs::remove_dir_all(dir).ok();
}
