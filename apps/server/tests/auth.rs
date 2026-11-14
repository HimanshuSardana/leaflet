//! Register endpoint behaviour: bootstrap admin, closed registration, invite
//! codes, and the cookie/token pair handed back on success.

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use leaflet::config::Config;
use leaflet::{AppState, router};
use serde_json::json;
use tower::ServiceExt;

async fn state(invite_code: Option<&str>) -> (AppState, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("leaflet-register-{}", uuid::Uuid::now_v7()));
    let config = Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        database_url: format!("sqlite://{}?mode=rwc", dir.join("leaflet.db").display()),
        data_dir: dir.clone(),
        web_dir: dir.join("web"),
        invite_code: invite_code.map(str::to_owned),
        cookie_secure: false,
        log: "error".into(),
    };
    let pool = leaflet::db::open(&config).await.unwrap();
    let storage = leaflet::storage::Storage::new(config.blobs_dir()).unwrap();
    let assets = leaflet::assets::Assets::new(&config.web_dir);
    let signing = leaflet::auth::tokens::SigningKey::from_bytes(&[5u8; 64]).unwrap();
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
    body: serde_json::Value,
    set_cookie: Option<String>,
}

async fn post(state: &AppState, path: &str, body: serde_json::Value) -> Reply {
    let response = router(state.clone())
        .oneshot(
            Request::post(path)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::USER_AGENT, "test-agent")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let set_cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    Reply {
        status,
        body,
        set_cookie,
    }
}

fn registration(email: &str, password: &str, invite: Option<&str>) -> serde_json::Value {
    json!({
        "email": email,
        "display_name": "Reader",
        "password": password,
        "invite_code": invite,
    })
}

#[tokio::test]
async fn first_registration_becomes_admin_and_signs_in() {
    let (state, dir) = state(None).await;
    let reply = post(
        &state,
        "/api/v1/auth/register",
        registration("admin@example.com", "hunter2hunter2", None),
    )
    .await;

    assert_eq!(reply.status, StatusCode::CREATED);
    assert_eq!(reply.body["user"]["role"], "admin");
    assert_eq!(reply.body["user"]["email"], "admin@example.com");
    assert!(reply.body["access_token"].as_str().unwrap().len() > 20);
    assert!(reply.body.get("password_hash").is_none());

    let cookie = reply.set_cookie.expect("refresh cookie is set");
    assert!(cookie.starts_with("leaflet_session="));
    assert!(cookie.contains("HttpOnly"));
    assert!(cookie.contains("SameSite=Strict"));

    std::fs::remove_dir_all(dir).ok();
}

#[tokio::test]
async fn registration_is_closed_once_an_account_exists() {
    let (state, dir) = state(None).await;
    post(
        &state,
        "/api/v1/auth/register",
        registration("admin@example.com", "hunter2hunter2", None),
    )
    .await;

    let reply = post(
        &state,
        "/api/v1/auth/register",
        registration("second@example.com", "hunter2hunter2", None),
    )
    .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    assert_eq!(reply.body["type"], "forbidden");
    std::fs::remove_dir_all(dir).ok();
}

#[tokio::test]
async fn invite_code_opens_registration_for_members() {
    let (state, dir) = state(Some("let-me-in")).await;
    post(
        &state,
        "/api/v1/auth/register",
        registration("admin@example.com", "hunter2hunter2", None),
    )
    .await;

    let wrong = post(
        &state,
        "/api/v1/auth/register",
        registration("b@example.com", "hunter2hunter2", Some("nope")),
    )
    .await;
    assert_eq!(wrong.status, StatusCode::FORBIDDEN);

    let right = post(
        &state,
        "/api/v1/auth/register",
        registration("b@example.com", "hunter2hunter2", Some("let-me-in")),
    )
    .await;
    assert_eq!(right.status, StatusCode::CREATED);
    assert_eq!(right.body["user"]["role"], "member");
    std::fs::remove_dir_all(dir).ok();
}

#[tokio::test]
async fn invalid_payloads_are_rejected_before_touching_the_db() {
    let (state, dir) = state(None).await;

    let bad_email = post(
        &state,
        "/api/v1/auth/register",
        registration("not-an-email", "hunter2hunter2", None),
    )
    .await;
    assert_eq!(bad_email.status, StatusCode::BAD_REQUEST);

    let short = post(
        &state,
        "/api/v1/auth/register",
        registration("a@example.com", "short", None),
    )
    .await;
    assert_eq!(short.status, StatusCode::BAD_REQUEST);
    assert!(short.body["detail"].as_str().unwrap().contains("at least"));
    assert_eq!(short.body["type"], "invalid_request");

    std::fs::remove_dir_all(dir).ok();
}

async fn login(state: &AppState, email: &str, password: &str) -> Reply {
    post(
        state,
        "/api/v1/auth/login",
        json!({ "email": email, "password": password }),
    )
    .await
}

#[tokio::test]
async fn login_with_the_right_password_returns_a_token_and_cookie() {
    let (state, dir) = state(None).await;
    post(
        &state,
        "/api/v1/auth/register",
        registration("reader@example.com", "hunter2hunter2", None),
    )
    .await;

    let reply = login(&state, "READER@example.com", "hunter2hunter2").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body["user"]["email"], "reader@example.com");
    assert!(reply.set_cookie.unwrap().contains("HttpOnly"));
    std::fs::remove_dir_all(dir).ok();
}

#[tokio::test]
async fn wrong_password_and_unknown_email_look_identical() {
    let (state, dir) = state(None).await;
    post(
        &state,
        "/api/v1/auth/register",
        registration("reader@example.com", "hunter2hunter2", None),
    )
    .await;

    let wrong = login(&state, "reader@example.com", "wrong-password").await;
    let unknown = login(&state, "nobody@example.com", "wrong-password").await;

    assert_eq!(wrong.status, StatusCode::UNAUTHORIZED);
    assert_eq!(unknown.status, StatusCode::UNAUTHORIZED);
    assert_eq!(wrong.body, unknown.body);
    assert!(wrong.set_cookie.is_none());
    std::fs::remove_dir_all(dir).ok();
}

#[tokio::test]
async fn version_reports_the_registration_mode() {
    let (open_state, open_dir) = state(None).await;
    let open = get_json(&open_state, "/api/v1/version").await;
    assert_eq!(open["registration"], "open");
    std::fs::remove_dir_all(open_dir).ok();

    let (closed_state, closed_dir) = state(None).await;
    post(
        &closed_state,
        "/api/v1/auth/register",
        registration("admin@example.com", "hunter2hunter2", None),
    )
    .await;
    let closed = get_json(&closed_state, "/api/v1/version").await;
    assert_eq!(closed["registration"], "closed");
    std::fs::remove_dir_all(closed_dir).ok();

    let (invite_state, invite_dir) = state(Some("let-me-in")).await;
    post(
        &invite_state,
        "/api/v1/auth/register",
        registration("admin@example.com", "hunter2hunter2", None),
    )
    .await;
    let invite = get_json(&invite_state, "/api/v1/version").await;
    assert_eq!(invite["registration"], "invite");
    std::fs::remove_dir_all(invite_dir).ok();
}

async fn get_json(state: &AppState, path: &str) -> serde_json::Value {
    let response = router(state.clone())
        .oneshot(Request::get(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

fn cookie_value(set_cookie: &str) -> String {
    set_cookie
        .split(';')
        .next()
        .unwrap()
        .split_once('=')
        .unwrap()
        .1
        .to_owned()
}

async fn send(state: &AppState, request: Request<Body>) -> Reply {
    let response = router(state.clone()).oneshot(request).await.unwrap();
    let status = response.status();
    let set_cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    Reply {
        status,
        body: serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
        set_cookie,
    }
}

#[tokio::test]
async fn refresh_rotates_the_cookie_and_mints_an_access_token() {
    let (state, dir) = state(None).await;
    let registered = post(
        &state,
        "/api/v1/auth/register",
        registration("reader@example.com", "hunter2hunter2", None),
    )
    .await;
    let first = cookie_value(registered.set_cookie.as_ref().unwrap());

    let reply = send(
        &state,
        Request::post("/api/v1/auth/refresh")
            .header(header::COOKIE, format!("leaflet_session={first}"))
            .header(header::HOST, "localhost:8080")
            .header(header::ORIGIN, "http://localhost:8080")
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(reply.status, StatusCode::OK);
    assert!(reply.body["access_token"].as_str().unwrap().len() > 20);

    let rotated = cookie_value(reply.set_cookie.as_ref().unwrap());
    assert_ne!(rotated, first, "the old cookie must stop working");

    let replay = send(
        &state,
        Request::post("/api/v1/auth/refresh")
            .header(header::COOKIE, format!("leaflet_session={first}"))
            .header(header::HOST, "localhost:8080")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(replay.status, StatusCode::UNAUTHORIZED);
    std::fs::remove_dir_all(dir).ok();
}

#[tokio::test]
async fn cross_origin_refresh_is_refused() {
    let (state, dir) = state(None).await;
    let registered = post(
        &state,
        "/api/v1/auth/register",
        registration("reader@example.com", "hunter2hunter2", None),
    )
    .await;
    let cookie = cookie_value(registered.set_cookie.as_ref().unwrap());

    let reply = send(
        &state,
        Request::post("/api/v1/auth/refresh")
            .header(header::COOKIE, format!("leaflet_session={cookie}"))
            .header(header::HOST, "leaflet.example")
            .header(header::ORIGIN, "https://evil.example")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    std::fs::remove_dir_all(dir).ok();
}

#[tokio::test]
async fn me_requires_a_bearer_token() {
    let (state, dir) = state(None).await;
    let registered = post(
        &state,
        "/api/v1/auth/register",
        registration("reader@example.com", "hunter2hunter2", None),
    )
    .await;
    let token = registered.body["access_token"].as_str().unwrap().to_owned();

    let anonymous = send(
        &state,
        Request::get("/api/v1/me").body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(anonymous.status, StatusCode::UNAUTHORIZED);
    assert_eq!(anonymous.body["type"], "unauthorized");

    let authenticated = send(
        &state,
        Request::get("/api/v1/me")
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(authenticated.status, StatusCode::OK);
    assert_eq!(authenticated.body["email"], "reader@example.com");
    assert!(authenticated.body.get("password_hash").is_none());
    std::fs::remove_dir_all(dir).ok();
}

#[tokio::test]
async fn logout_revokes_the_session_and_clears_the_cookie() {
    let (state, dir) = state(None).await;
    let registered = post(
        &state,
        "/api/v1/auth/register",
        registration("reader@example.com", "hunter2hunter2", None),
    )
    .await;
    let cookie = cookie_value(registered.set_cookie.as_ref().unwrap());

    let reply = send(
        &state,
        Request::post("/api/v1/auth/logout")
            .header(header::COOKIE, format!("leaflet_session={cookie}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);
    let cleared = reply.set_cookie.unwrap();
    assert!(cleared.contains("Max-Age=0"));

    let refresh = send(
        &state,
        Request::post("/api/v1/auth/refresh")
            .header(header::COOKIE, format!("leaflet_session={cookie}"))
            .header(header::HOST, "localhost:8080")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(refresh.status, StatusCode::UNAUTHORIZED);
    std::fs::remove_dir_all(dir).ok();
}
