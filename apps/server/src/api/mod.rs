pub mod version;

use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
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
    pub database: &'static str,
}

async fn healthz(State(state): State<AppState>) -> Response {
    match crate::db::ping(&state.pool).await {
        Ok(()) => (
            StatusCode::OK,
            axum::Json(Health {
                status: "ok",
                database: "ok",
            }),
        )
            .into_response(),
        Err(err) => {
            tracing::error!(%err, "health check failed");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                axum::Json(Health {
                    status: "degraded",
                    database: "unavailable",
                }),
            )
                .into_response()
        }
    }
}
