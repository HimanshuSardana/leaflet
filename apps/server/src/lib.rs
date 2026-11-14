pub mod api;
pub mod assets;
pub mod auth;
pub mod config;
pub mod db;
pub mod domain;
pub mod openapi;
pub mod storage;

use axum::Router;
use sqlx::SqlitePool;
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct AppState {
    pub config: config::Config,
    pub pool: SqlitePool,
    pub storage: storage::Storage,
    pub assets: assets::Assets,
}

pub fn router(state: AppState) -> Router {
    let assets = state.assets.clone();
    Router::new()
        .merge(api::routes())
        .fallback(move |uri: axum::http::Uri| {
            let assets = assets.clone();
            async move { assets.serve(uri).await }
        })
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
