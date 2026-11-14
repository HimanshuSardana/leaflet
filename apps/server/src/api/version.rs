use axum::Router;
use axum::routing::get;
use serde::Serialize;
use utoipa::ToSchema;

use crate::AppState;

pub const API_VERSION: &str = "v1";

pub fn routes() -> Router<AppState> {
    Router::new().route("/version", get(version))
}

#[derive(Debug, Serialize, ToSchema)]
pub struct Version {
    pub name: &'static str,
    pub version: &'static str,
    pub api: &'static str,
    /// Schema revision this server migrated to; clients refuse to sync above it.
    pub schema: &'static str,
}

#[utoipa::path(get, path = "/api/v1/version", tag = "meta", responses(
    (status = 200, description = "server identity and contract version", body = Version)
))]
pub(crate) async fn version() -> axum::Json<Version> {
    axum::Json(Version {
        name: env!("CARGO_PKG_NAME"),
        version: env!("CARGO_PKG_VERSION"),
        api: API_VERSION,
        schema: crate::domain::SCHEMA_REVISION,
    })
}
