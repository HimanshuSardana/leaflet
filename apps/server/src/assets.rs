//! Serves the exported Next.js bundle from the same origin as the API, so the
//! browser never needs CORS and the API url is never baked into the client.
//! `/api/*` and `/healthz` never fall through to index.html: a mistyped api
//! path must 404, not hand the SPA shell to a fetch().

use std::path::PathBuf;

use axum::http::{StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use tower_http::services::ServeDir;

#[derive(Debug, Clone)]
pub struct Assets {
    dir: PathBuf,
}

impl Assets {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn dir(&self) -> &PathBuf {
        &self.dir
    }

    /// `None` when no export has been built; the server then serves api only.
    pub fn resolved(&self) -> Option<ServeDir> {
        self.dir
            .is_dir()
            .then(|| ServeDir::new(&self.dir).append_index_html_on_directories(true))
    }

    pub async fn serve(&self, uri: Uri) -> Response {
        let path = uri.path();
        if path.starts_with("/api/") || path == "/healthz" {
            return (StatusCode::NOT_FOUND, "not found").into_response();
        }
        match self.resolved() {
            Some(service) => match tower::ServiceExt::oneshot(service, request_for(uri)).await {
                Ok(response) if response.status() != StatusCode::NOT_FOUND => {
                    response.into_response()
                }
                _ => (StatusCode::NOT_FOUND, "not found").into_response(),
            },
            // No export: answer the root with a build hint, everything else 404.
            None if path == "/" => (
                StatusCode::SERVICE_UNAVAILABLE,
                "web assets are not built: run `pnpm --filter web build`",
            )
                .into_response(),
            None => (StatusCode::NOT_FOUND, "not found").into_response(),
        }
    }
}

fn request_for(uri: Uri) -> axum::http::Request<axum::body::Body> {
    axum::http::Request::builder()
        .uri(uri)
        .body(axum::body::Body::empty())
        .expect("static request is well formed")
}
