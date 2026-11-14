pub mod api;
pub mod config;
pub mod domain;

use axum::Router;
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct AppState {
    pub config: config::Config,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(api::routes())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
