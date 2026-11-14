pub mod api;
pub mod config;
pub mod db;
pub mod domain;
pub mod openapi;

use axum::Router;
use sqlx::SqlitePool;
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct AppState {
    pub config: config::Config,
    pub pool: SqlitePool,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(api::routes())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
