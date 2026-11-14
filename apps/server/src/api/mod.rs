pub mod auth;
pub mod books;
pub mod error;
pub mod version;

use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use serde::Serialize;
use utoipa::{OpenApi, ToSchema};

use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/healthz", get(healthz)).nest(
        "/api/v1",
        version::routes()
            .merge(auth::routes())
            .merge(books::routes())
            .merge(me_route()),
    )
}

fn me_route() -> Router<AppState> {
    Router::new().route("/me", get(auth::me))
}

#[derive(Serialize, ToSchema)]
pub struct Health {
    pub status: &'static str,
    pub database: &'static str,
}

/// Liveness + readiness: 200 only while the database answers.
#[utoipa::path(get, path = "/healthz", tag = "meta", responses(
    (status = 200, description = "server and database are up", body = Health),
    (status = 503, description = "database unavailable", body = Health)
))]
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

#[derive(OpenApi)]
#[openapi(
    info(
        title = "leaflet",
        description = "Self-hostable ebook, manga and light novel server",
        license(name = "AGPL-3.0-only")
    ),
    paths(
        healthz,
        version::version,
        auth::register,
        auth::login,
        auth::refresh,
        auth::logout,
        auth::me,
        books::list,
        books::upload,
        books::get_one,
        books::download
    ),
    components(schemas(
        Health,
        version::Version,
        error::Problem,
        auth::RegisterRequest,
        auth::AuthSession,
        auth::LoginRequest,
        auth::AccessToken,
        crate::domain::user::User,
        crate::domain::user::Role,
        crate::domain::RegistrationMode,
        crate::domain::Book,
        crate::domain::BookFormat,
    ))
)]
pub struct ApiDoc;
