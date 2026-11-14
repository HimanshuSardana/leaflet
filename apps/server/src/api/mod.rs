pub mod version;

use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use serde::Serialize;

use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/healthz", get(healthz))
        .nest("/api/v1", version::routes())
}

#[derive(Serialize)]
pub struct Health {
    pub status: &'static str,
}

async fn healthz(State(_state): State<AppState>) -> (StatusCode, axum::Json<Health>) {
    (StatusCode::OK, axum::Json(Health { status: "ok" }))
}
