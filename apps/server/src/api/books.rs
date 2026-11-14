//! Upload and read books.
//!
//! Order matters: the bytes are streamed into content-addressed storage first,
//! the row second. A crash between the two costs an orphan blob (harmless, the
//! next upload of the same file reuses it) but can never leave a row pointing at
//! nothing. A rejected upload has no row at all, so its blob is removed again.

use axum::Json;
use axum::Router;
use axum::body::Body;
use axum::extract::multipart::Multipart;
use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};
use tokio_util::io::ReaderStream;
use uuid::Uuid;

use crate::AppState;
use crate::api::error::ApiError;
use crate::auth::extract::CurrentUser;
use crate::db::books::{self, NewBook};
use crate::domain::Book;
use crate::domain::book::{sniff_format, title_from_filename};

pub fn routes() -> Router<AppState> {
    Router::new()
        // Uploads are streamed and bounded by config, so axum's 2 MB body limit
        // (which would reject every book) is lifted for this route alone.
        .route(
            "/books",
            get(list).merge(post(upload).layer(DefaultBodyLimit::disable())),
        )
        .route("/books/{id}", get(get_one))
        .route("/books/{id}/file", get(download))
}

/// Lists the caller's shelf, alphabetically by sort title.
#[utoipa::path(get, path = "/api/v1/books", tag = "books",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "the caller's books", body = Vec<Book>),
        (status = 401, description = "missing or expired access token", body = crate::api::error::Problem)
    ))]
pub async fn list(State(state): State<AppState>, user: CurrentUser) -> Response {
    match books::list(&state.pool, user.id).await {
        Ok(books) => Json(books).into_response(),
        Err(err) => {
            tracing::error!(%err, "listing books failed");
            ApiError::internal().into_response()
        }
    }
}

#[utoipa::path(get, path = "/api/v1/books/{id}", tag = "books",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "book id")),
    responses(
        (status = 200, description = "the book", body = Book),
        (status = 404, description = "no such book, or not this user's", body = crate::api::error::Problem)
    ))]
pub async fn get_one(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Response {
    match books::get(&state.pool, user.id, id).await {
        Ok(Some(book)) => Json(book).into_response(),
        Ok(None) => ApiError::not_found().into_response(),
        Err(err) => {
            tracing::error!(%err, "reading book failed");
            ApiError::internal().into_response()
        }
    }
}

/// Uploads one PDF or EPUB as `multipart/form-data` with a field named `file`.
///
/// The format is sniffed from the bytes, so a mislabelled file is rejected
/// instead of being stored as something it is not. Uploading the same bytes
/// twice is a no-op that answers 200 with the existing book; new books answer
/// 201.
#[utoipa::path(post, path = "/api/v1/books", tag = "books",
    security(("bearer" = [])),
    request_body(content = String, content_type = "multipart/form-data",
        description = "one part, name `file`, containing the PDF or EPUB"),
    responses(
        (status = 201, description = "book stored", body = Book),
        (status = 200, description = "this copy was already in the library", body = Book),
        (status = 400, description = "malformed or empty upload", body = crate::api::error::Problem),
        (status = 401, description = "missing or expired access token", body = crate::api::error::Problem),
        (status = 413, description = "upload is larger than the configured limit", body = crate::api::error::Problem),
        (status = 415, description = "the bytes are not a PDF or EPUB", body = crate::api::error::Problem)
    ))]
pub async fn upload(
    State(state): State<AppState>,
    user: CurrentUser,
    mut multipart: Multipart,
) -> Response {
    let field = loop {
        match multipart.next_field().await {
            Ok(Some(field)) if field.name() == Some("file") => break field,
            Ok(Some(_)) => continue,
            Ok(None) => {
                return ApiError::validation("expected a multipart field named `file`")
                    .into_response();
            }
            Err(err) => {
                return ApiError::bad_request("malformed multipart body")
                    .with_detail(err.to_string())
                    .into_response();
            }
        }
    };

    let filename = field.file_name().unwrap_or_default().to_owned();

    let stored = match state
        .storage
        .put_stream(Box::pin(field), state.config.max_upload_bytes)
        .await
    {
        Ok(stored) => stored,
        Err(err) => return storage_error(err, state.config.max_upload_bytes),
    };

    let some = |reason: ApiError| async {
        // Nothing references these bytes yet, so they must not linger.
        if let Err(err) = state
            .storage
            .delete_if_unreferenced(&state.pool, &stored.sha256)
            .await
        {
            tracing::warn!(%err, sha256 = %stored.sha256, "could not clean up a rejected upload");
        }
        reason.into_response()
    };

    if stored.byte_size == 0 {
        return some(ApiError::validation("the uploaded file is empty")).await;
    }

    let Some(format) = sniff_format(&stored.head) else {
        return some(
            ApiError::new(
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "unsupported_format",
                "Not a book leaflet can read",
            )
            .with_detail("only PDF and EPUB are supported"),
        )
        .await;
    };

    let title = title_from_filename(&filename);
    let new = NewBook {
        owner_id: user.id,
        title: &title,
        author: None,
        format,
        sha256: &stored.sha256,
        byte_size: stored.byte_size as i64,
    };

    match books::insert(&state.pool, new).await {
        Ok((book, created)) => {
            let status = if created {
                StatusCode::CREATED
            } else {
                StatusCode::OK
            };
            let location = format!("/api/v1/books/{}", book.id);
            tracing::info!(
                book = %book.id, title = %book.title, format = format.as_str(),
                bytes = book.byte_size, deduplicated = stored.deduplicated,
                "book uploaded"
            );
            (status, [(header::LOCATION, location)], Json(book)).into_response()
        }
        Err(err) => {
            tracing::error!(%err, "storing book failed");
            ApiError::internal().into_response()
        }
    }
}

/// Serves the stored file. Byte ranges are honoured because PDF readers seek
/// into the file instead of downloading it whole.
#[utoipa::path(get, path = "/api/v1/books/{id}/file", tag = "books",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "book id")),
    responses(
        (status = 200, description = "the whole file"),
        (status = 206, description = "the requested byte range"),
        (status = 404, description = "no such book, or not this user's", body = crate::api::error::Problem),
        (status = 416, description = "range not satisfiable")
    ))]
pub async fn download(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Response {
    let book = match books::get(&state.pool, user.id, id).await {
        Ok(Some(book)) => book,
        Ok(None) => return ApiError::not_found().into_response(),
        Err(err) => {
            tracing::error!(%err, "reading book failed");
            return ApiError::internal().into_response();
        }
    };

    let path = state.storage.path_for(&book.sha256);
    let file = match tokio::fs::File::open(&path).await {
        Ok(file) => file,
        Err(err) => {
            tracing::error!(%err, sha256 = %book.sha256, "blob missing for book");
            return ApiError::internal().into_response();
        }
    };
    let len = match tokio::fs::metadata(&path).await {
        Ok(meta) => meta.len(),
        Err(err) => {
            tracing::error!(%err, sha256 = %book.sha256, "blob unreadable for book");
            return ApiError::internal().into_response();
        }
    };

    let etag = format!("\"{}\"", book.sha256);
    let mut response_headers = HeaderMap::new();
    // hex sha and a sanitised filename are always valid header values
    response_headers.insert(header::ETAG, HeaderValue::from_str(&etag).expect("etag"));
    response_headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(book.format.content_type()),
    );
    response_headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=31536000, immutable"),
    );
    response_headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&content_disposition(&book)).expect("content disposition"),
    );

    // Content addressed bytes never change, so a matching tag saves the body.
    if headers
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.split(',').any(|tag| tag.trim() == etag))
    {
        response_headers.insert(header::CONTENT_LENGTH, HeaderValue::from_static("0"));
        return (StatusCode::NOT_MODIFIED, response_headers).into_response();
    }

    let requested = parse_range(
        headers
            .get(header::RANGE)
            .and_then(|value| value.to_str().ok()),
        len,
    );

    let (status, start, count) = match requested {
        RangeRequest::Satisfiable(range) => {
            let count = range.end - range.start + 1;
            response_headers.insert(
                header::CONTENT_RANGE,
                HeaderValue::from_str(&format!("bytes {}-{}/{}", range.start, range.end, len))
                    .expect("content range"),
            );
            (StatusCode::PARTIAL_CONTENT, range.start, count)
        }
        RangeRequest::Unsatisfiable => {
            response_headers.insert(
                header::CONTENT_RANGE,
                HeaderValue::from_str(&format!("bytes */{len}")).expect("content range"),
            );
            response_headers.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
            return (StatusCode::RANGE_NOT_SATISFIABLE, response_headers).into_response();
        }
        RangeRequest::None => (StatusCode::OK, 0, len),
    };

    response_headers.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    response_headers.insert(
        header::CONTENT_LENGTH,
        HeaderValue::from_str(&count.to_string()).expect("content length"),
    );

    let mut file = file;
    if start > 0 && file.seek(SeekFrom::Start(start)).await.is_err() {
        return ApiError::internal().into_response();
    }
    let body = Body::from_stream(ReaderStream::new(file.take(count)));

    (status, response_headers, body).into_response()
}

/// `inline` so a PDF opens in the browser's viewer; the filename is reduced to
/// ASCII word characters because it goes into a quoted header value.
fn content_disposition(book: &Book) -> String {
    let mut name: String = book
        .title
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == ' ' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    name.truncate(80);
    let name = name.trim();
    let name = if name.is_empty() { "book" } else { name };
    format!("inline; filename=\"{name}.{}\"", book.format.extension())
}

fn storage_error(err: crate::storage::StorageError, max: u64) -> Response {
    match err {
        crate::storage::StorageError::TooLarge(_) => ApiError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "payload_too_large",
            "Upload is too large",
        )
        .with_detail(format!("the limit is {max} bytes"))
        .into_response(),
        crate::storage::StorageError::Upload(detail) => {
            ApiError::bad_request("upload was cut short")
                .with_detail(detail)
                .into_response()
        }
        other => {
            tracing::error!(%other, "storing upload failed");
            ApiError::internal().into_response()
        }
    }
}

struct ByteRange {
    start: u64,
    end: u64,
}

enum RangeRequest {
    None,
    Satisfiable(ByteRange),
    Unsatisfiable,
}

/// Only single ranges are honoured; a multi-range request is answered with the
/// whole file, which is a legal (if lazy) response. Suffix ranges (`-N`, the
/// last N bytes) are what PDF viewers use to read a trailer.
fn parse_range(raw: Option<&str>, len: u64) -> RangeRequest {
    let Some(raw) = raw else {
        return RangeRequest::None;
    };
    let Some(spec) = raw.trim().strip_prefix("bytes=") else {
        return RangeRequest::None;
    };
    let spec = spec.trim();
    if spec.contains(',') {
        return RangeRequest::None;
    }
    let Some((first, last)) = spec.split_once('-') else {
        return RangeRequest::None;
    };
    let (first, last) = (first.trim(), last.trim());

    if first.is_empty() {
        let Ok(suffix) = last.parse::<u64>() else {
            return RangeRequest::Unsatisfiable;
        };
        if suffix == 0 || len == 0 {
            return RangeRequest::Unsatisfiable;
        }
        let suffix = suffix.min(len);
        return RangeRequest::Satisfiable(ByteRange {
            start: len - suffix,
            end: len - 1,
        });
    }

    let Ok(start) = first.parse::<u64>() else {
        return RangeRequest::Unsatisfiable;
    };
    if start >= len {
        return RangeRequest::Unsatisfiable;
    }
    let end = if last.is_empty() {
        len - 1
    } else {
        match last.parse::<u64>() {
            Ok(end) => end.min(len - 1),
            Err(_) => return RangeRequest::Unsatisfiable,
        }
    };
    if end < start {
        return RangeRequest::Unsatisfiable;
    }
    RangeRequest::Satisfiable(ByteRange { start, end })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_cover_prefix_suffix_and_open_ends() {
        assert!(matches!(parse_range(None, 100), RangeRequest::None));
        match parse_range(Some("bytes=0-3"), 100) {
            RangeRequest::Satisfiable(range) => assert_eq!((range.start, range.end), (0, 3)),
            _ => panic!("expected a satisfiable range"),
        }
        match parse_range(Some("bytes=10-"), 100) {
            RangeRequest::Satisfiable(range) => assert_eq!((range.start, range.end), (10, 99)),
            _ => panic!("expected an open-ended range"),
        }
        match parse_range(Some("bytes=-20"), 100) {
            RangeRequest::Satisfiable(range) => assert_eq!((range.start, range.end), (80, 99)),
            _ => panic!("expected a suffix range"),
        }
        match parse_range(Some("bytes=90-500"), 100) {
            RangeRequest::Satisfiable(range) => assert_eq!((range.start, range.end), (90, 99)),
            _ => panic!("end is clamped to the file"),
        }
        assert!(matches!(
            parse_range(Some("bytes=100-"), 100),
            RangeRequest::Unsatisfiable
        ));
        assert!(matches!(
            parse_range(Some("bytes=5-1"), 100),
            RangeRequest::Unsatisfiable
        ));
        assert!(matches!(
            parse_range(Some("bytes=abc-1"), 100),
            RangeRequest::Unsatisfiable
        ));
        assert!(matches!(
            parse_range(Some("bytes=0-1,5-6"), 100),
            RangeRequest::None
        ));
        assert!(matches!(
            parse_range(Some("items=0-1"), 100),
            RangeRequest::None
        ));
    }
}
