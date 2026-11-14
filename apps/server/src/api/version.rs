use axum::Router;
use axum::extract::State;
use axum::routing::get;
use serde::Serialize;
use utoipa::ToSchema;

use crate::AppState;
use crate::domain::RegistrationMode;

/// Public by design: the client needs it to decide what to render, and it
/// reveals nothing an attacker cannot learn from POSTing to /register.
pub async fn registration_mode(state: &AppState) -> RegistrationMode {
    match crate::db::users::count(&state.pool).await {
        Ok(0) => RegistrationMode::Open,
        Ok(_) if state.config.invite_code.is_some() => RegistrationMode::Invite,
        Ok(_) => RegistrationMode::Closed,
        Err(err) => {
            tracing::error!(%err, "cannot read user count");
            RegistrationMode::Closed
        }
    }
}

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
    /// Whether a client may register, and whether it needs an invite code.
    pub registration: crate::domain::RegistrationMode,
}

#[utoipa::path(get, path = "/api/v1/version", tag = "meta", responses(
    (status = 200, description = "server identity and contract version", body = Version)
))]
pub(crate) async fn version(State(state): State<AppState>) -> axum::Json<Version> {
    axum::Json(Version {
        name: env!("CARGO_PKG_NAME"),
        version: env!("CARGO_PKG_VERSION"),
        api: API_VERSION,
        schema: crate::domain::SCHEMA_REVISION,
        registration: registration_mode(&state).await,
    })
}
