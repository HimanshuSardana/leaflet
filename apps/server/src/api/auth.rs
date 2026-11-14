//! Registration and login. The first account bootstraps as admin; after that
//! registration needs the configured invite code, so a public server is closed
//! by default.

use axum::Json;
use axum::extract::State;
use axum::http::header::{HOST, ORIGIN};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum_extra::extract::cookie::CookieJar;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::AppState;
use crate::api::error::ApiError;
use crate::auth::extract::CurrentUser;
use crate::auth::{cookie, hash_password, tokens, verify_password};
use crate::db::{sessions, users};
use crate::domain::user::{Role, User};

#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterRequest {
    pub email: String,
    pub display_name: String,
    pub password: String,
    /// Required once an account exists; ignored for the bootstrap account.
    #[serde(default)]
    pub invite_code: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AccessToken {
    pub access_token: String,
    pub access_expires_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuthSession {
    pub user: User,
    /// Short-lived; the client keeps this in memory and refreshes via cookie.
    pub access_token: String,
    pub access_expires_at: chrono::DateTime<Utc>,
}

#[utoipa::path(post, path = "/api/v1/auth/register", tag = "auth",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "account created and signed in", body = AuthSession),
        (status = 400, description = "invalid email, name or password", body = crate::api::error::Problem),
        (status = 403, description = "registration closed or invite code wrong", body = crate::api::error::Problem),
        (status = 409, description = "email already registered", body = crate::api::error::Problem)
    ))]
pub async fn register(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<RegisterRequest>,
) -> Response {
    let email = body.email.trim();
    if !looks_like_email(email) {
        return ApiError::validation("email is not valid").into_response();
    }
    let display_name = body.display_name.trim();
    if display_name.is_empty() || display_name.chars().count() > 64 {
        return ApiError::validation("display_name must be 1-64 characters").into_response();
    }

    let existing = match users::count(&state.pool).await {
        Ok(count) => count,
        Err(err) => {
            tracing::error!(%err, "cannot count users");
            return ApiError::internal().into_response();
        }
    };

    let role = if existing == 0 {
        Role::Admin
    } else {
        match &state.config.invite_code {
            Some(expected) if body.invite_code.as_deref() == Some(expected.as_str()) => {
                Role::Member
            }
            Some(_) => {
                return ApiError::forbidden()
                    .with_detail("invalid invite code")
                    .into_response();
            }
            None => {
                return ApiError::forbidden()
                    .with_detail("registration is closed on this server")
                    .into_response();
            }
        }
    };

    let hash = match hash_password(&body.password) {
        Ok(hash) => hash,
        Err(err) => return ApiError::validation(err.to_string()).into_response(),
    };

    let user = match users::insert(
        &state.pool,
        users::NewUser {
            email,
            display_name,
            password_hash: &hash,
            role,
        },
    )
    .await
    {
        Ok(user) => user,
        Err(users::UserError::EmailTaken) => {
            return ApiError::conflict("email already registered").into_response();
        }
        Err(err) => {
            tracing::error!(%err, "cannot create user");
            return ApiError::internal().into_response();
        }
    };

    let now = Utc::now();
    let issued =
        match sessions::issue(&state.pool, user.id, user_agent(&headers).as_deref(), now).await {
            Ok(issued) => issued,
            Err(err) => {
                tracing::error!(%err, "cannot issue session");
                return ApiError::internal().into_response();
            }
        };

    signed_in(
        StatusCode::CREATED,
        state.config.cookie_secure,
        jar,
        user,
        issued,
        &state.signing,
        now,
    )
}

#[utoipa::path(post, path = "/api/v1/auth/login", tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "signed in", body = AuthSession),
        (status = 401, description = "wrong email or password", body = crate::api::error::Problem)
    ))]
pub async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<LoginRequest>,
) -> Response {
    let now = Utc::now();
    let record = match users::find_by_email(&state.pool, &body.email).await {
        Ok(record) => record,
        Err(err) => {
            tracing::error!(%err, "cannot look up user");
            return ApiError::internal().into_response();
        }
    };

    // Always run one verification so a missing account and a wrong password
    // take the same time; the dummy hash is built once, lazily.
    let (user, verified) = match record {
        Some(record) => {
            let verified = verify_password(&body.password, &record.password_hash).unwrap_or(false);
            (record.into_user().ok_or_else(ApiError::internal), verified)
        }
        None => {
            let _ = verify_password(&body.password, dummy_hash());
            (Err(ApiError::internal()), false)
        }
    };

    if !verified {
        return ApiError::new(
            StatusCode::UNAUTHORIZED,
            "invalid_credentials",
            "Invalid credentials",
        )
        .with_detail("email or password is wrong")
        .into_response();
    }
    let Ok(user) = user else {
        return ApiError::internal().into_response();
    };

    let issued =
        match sessions::issue(&state.pool, user.id, user_agent(&headers).as_deref(), now).await {
            Ok(issued) => issued,
            Err(err) => {
                tracing::error!(%err, "cannot issue session");
                return ApiError::internal().into_response();
            }
        };

    signed_in(
        StatusCode::OK,
        state.config.cookie_secure,
        jar,
        user,
        issued,
        &state.signing,
        now,
    )
}

/// Shared finish line for register/login/refresh: access token in the body,
/// session token in the cookie.
fn signed_in(
    status: StatusCode,
    secure_cookie: bool,
    jar: CookieJar,
    user: User,
    issued: sessions::IssuedSession,
    signing: &tokens::SigningKey,
    now: chrono::DateTime<Utc>,
) -> Response {
    let access_expires_at = now + tokens::ACCESS_TTL;
    let access_token = signing.issue_access(user.id, user.role.as_str(), now);
    let max_age = issued.expires_at - now;

    let jar = jar.add(cookie::refresh_cookie(
        issued.session.as_str(),
        max_age,
        secure_cookie,
    ));
    let body = AuthSession {
        user,
        access_token,
        access_expires_at,
    };

    (status, jar, Json(body)).into_response()
}

pub fn user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.chars().take(120).collect())
}

/// Exchanges the refresh cookie for a new access token, rotating the session.
/// Browsers send `Origin` on POST; if it is present it must match the host the
/// request arrived on, which (with SameSite=Strict) closes the CSRF hole.
#[utoipa::path(post, path = "/api/v1/auth/refresh", tag = "auth",
    responses(
        (status = 200, description = "new access token", body = AccessToken),
        (status = 401, description = "no valid session cookie", body = crate::api::error::Problem)
    ))]
pub async fn refresh(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
) -> Response {
    if !same_origin(&headers) {
        return ApiError::forbidden()
            .with_detail("cross-origin refresh is not allowed")
            .into_response();
    }

    let Some(cookie) = jar.get(cookie::REFRESH_COOKIE) else {
        return ApiError::unauthorized()
            .with_detail("no session cookie")
            .into_response();
    };

    let now = Utc::now();
    let issued = match sessions::rotate(
        &state.pool,
        cookie.value(),
        user_agent(&headers).as_deref(),
        now,
    )
    .await
    {
        Ok(issued) => issued,
        Err(sessions::SessionError::Invalid) => {
            return ApiError::unauthorized()
                .with_detail("session expired or revoked")
                .into_response();
        }
        Err(err) => {
            tracing::error!(%err, "cannot rotate session");
            return ApiError::internal().into_response();
        }
    };

    let context = match sessions::resolve(&state.pool, issued.session.as_str(), now).await {
        Ok(context) => context,
        Err(err) => {
            tracing::error!(%err, "cannot resolve the rotated session");
            return ApiError::internal().into_response();
        }
    };

    let max_age = issued.expires_at - now;
    let access_expires_at = now + tokens::ACCESS_TTL;
    let access_token = state
        .signing
        .issue_access(context.user.id, context.user.role.as_str(), now);
    let jar = jar.add(cookie::refresh_cookie(
        issued.session.as_str(),
        max_age,
        state.config.cookie_secure,
    ));

    (
        StatusCode::OK,
        jar,
        Json(AccessToken {
            access_token,
            access_expires_at,
        }),
    )
        .into_response()
}

#[utoipa::path(post, path = "/api/v1/auth/logout", tag = "auth",
    responses((status = 204, description = "session revoked, cookie cleared")))]
pub async fn logout(State(state): State<AppState>, jar: CookieJar) -> Response {
    if let Some(cookie) = jar.get(cookie::REFRESH_COOKIE)
        && let Err(err) = sessions::revoke(&state.pool, cookie.value(), Utc::now()).await
    {
        tracing::error!(%err, "cannot revoke session");
    }
    let jar = jar.add(cookie::cleared_cookie(state.config.cookie_secure));
    (StatusCode::NO_CONTENT, jar).into_response()
}

/// The account behind the bearer token, loaded fresh so a role change or a
/// deleted account takes effect immediately.
#[utoipa::path(get, path = "/api/v1/me", tag = "auth",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "the signed-in user", body = User),
        (status = 401, description = "missing or expired access token", body = crate::api::error::Problem)
    ))]
pub async fn me(State(state): State<AppState>, current: CurrentUser) -> Response {
    match users::find_by_id(&state.pool, current.id).await {
        Ok(Some(user)) => Json(user).into_response(),
        Ok(None) => ApiError::unauthorized()
            .with_detail("account no longer exists")
            .into_response(),
        Err(err) => {
            tracing::error!(%err, "cannot load user");
            ApiError::internal().into_response()
        }
    }
}

/// Hash of a throwaway password, used to equalise login timing for unknown
/// emails. Computed once; argon2 is intentionally slow.
fn dummy_hash() -> &'static str {
    static DUMMY: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    DUMMY.get_or_init(|| {
        hash_password("not-a-real-password").expect("dummy hash must be constructible")
    })
}

/// Browsers send `Origin` on cross-site POSTs; a same-origin one matches `Host`.
/// A request with no `Origin` at all (curl, the Android client) is allowed: it
/// cannot be forged by a browser page.
fn same_origin(headers: &HeaderMap) -> bool {
    let Some(origin) = headers.get(ORIGIN).and_then(|value| value.to_str().ok()) else {
        return true;
    };
    // A proxy that rewrites the destination (next dev's rewrite, caddy, nginx)
    // reports the host the browser actually used here. A form POST cannot forge
    // it, and a fetch that tries needs a preflight this API never grants.
    let Some(host) = headers
        .get("x-forwarded-host")
        .or_else(|| headers.get(HOST))
        .and_then(|value| value.to_str().ok())
        .map(|value| value.split(',').next().unwrap_or(value).trim())
    else {
        return false;
    };
    origin
        .split("://")
        .nth(1)
        .map(|authority| authority == host)
        .unwrap_or(false)
}

fn looks_like_email(candidate: &str) -> bool {
    let mut parts = candidate.split('@');
    let (Some(local), Some(domain), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    !local.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !candidate.chars().any(char::is_whitespace)
        && candidate.len() <= 254
}

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/auth/register", axum::routing::post(register))
        .route("/auth/login", axum::routing::post(login))
        .route("/auth/refresh", axum::routing::post(refresh))
        .route("/auth/logout", axum::routing::post(logout))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(pairs: &[(&'static str, &str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (name, value) in pairs {
            map.insert(*name, value.parse().unwrap());
        }
        map
    }

    #[test]
    fn same_origin_trusts_the_proxys_host_header() {
        // the dev rewrite forwards the browser's host, not the api's
        assert!(same_origin(&headers(&[
            ("origin", "http://localhost:3000"),
            ("host", "127.0.0.1:8080"),
            ("x-forwarded-host", "localhost:3000"),
        ])));
        // a form post cannot set that header, so a real cross-site post is caught
        assert!(!same_origin(&headers(&[
            ("origin", "https://evil.example"),
            ("host", "leaflet.example"),
        ])));
        // first entry wins when proxies chain
        assert!(same_origin(&headers(&[
            ("origin", "https://leaflet.example"),
            ("host", "10.0.0.2:8080"),
            ("x-forwarded-host", "leaflet.example, 10.0.0.1"),
        ])));
        // clients without an Origin (curl, the android app) are not browser posts
        assert!(same_origin(&headers(&[("host", "leaflet.example")])));
        assert!(!same_origin(&headers(&[(
            "origin",
            "https://evil.example"
        )])));
    }

    #[test]
    fn email_shape_is_loose_but_not_silly() {
        assert!(looks_like_email("reader@example.com"));
        assert!(looks_like_email("a.b+tag@sub.example.co.uk"));
        assert!(!looks_like_email("reader"));
        assert!(!looks_like_email("reader@example"));
        assert!(!looks_like_email("@example.com"));
        assert!(!looks_like_email("reader@.com"));
        assert!(!looks_like_email("reader @example.com"));
        assert!(!looks_like_email("a@b@c.com"));
    }
}
