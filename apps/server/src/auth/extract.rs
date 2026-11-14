//! `CurrentUser` — the extractor every protected handler takes.
//!
//! Browser clients authenticate with the short-lived bearer access token
//! (minted by login/register/refresh, held in memory). The session cookie is
//! never accepted here: it only buys a new access token from /auth/refresh,
//! which keeps the cookie's blast radius to one endpoint.

use axum::extract::FromRequestParts;
use axum::http::StatusCode;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use uuid::Uuid;

use crate::AppState;
use crate::api::error::ApiError;
use crate::domain::user::Role;

pub struct CurrentUser {
    pub id: Uuid,
    pub role: Role,
}

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let raw = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .ok_or_else(ApiError::unauthorized)?;

        let claims = state
            .signing
            .verify_access(raw, chrono::Utc::now())
            .map_err(|_| {
                ApiError::new(
                    StatusCode::UNAUTHORIZED,
                    "invalid_token",
                    "Access token is invalid or expired",
                )
            })?;

        Ok(Self {
            id: Uuid::parse_str(&claims.sub).map_err(|_| ApiError::unauthorized())?,
            role: Role::parse(&claims.role).ok_or_else(ApiError::unauthorized)?,
        })
    }
}
