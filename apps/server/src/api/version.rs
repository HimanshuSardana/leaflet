use axum::Router;
use axum::routing::get;
use serde::Serialize;

use crate::AppState;

pub const API_VERSION: &str = "v1";

pub fn routes() -> Router<AppState> {
    Router::new().route("/version", get(version))
}

#[derive(Debug, Serialize)]
pub struct Version {
    pub name: &'static str,
    pub version: &'static str,
    pub api: &'static str,
    /// Highest reading_progress schema revision this server understands.
    pub schema: &'static str,
}

async fn version() -> axum::Json<Version> {
    axum::Json(Version {
        name: env!("CARGO_PKG_NAME"),
        version: env!("CARGO_PKG_VERSION"),
        api: API_VERSION,
        schema: crate::domain::SCHEMA_REVISION,
    })
}
