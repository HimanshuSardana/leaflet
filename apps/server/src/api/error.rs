//! RFC 9457 problem details. Every non-2xx the API returns has this shape, so
//! clients (web today, Android later) parse one error format.

use axum::Json;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;

pub const PROBLEM_JSON: &str = "application/problem+json";

#[derive(Debug, Serialize, ToSchema)]
pub struct Problem {
    /// Stable, machine-readable slug: `not_found`, `unauthorized`, …
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub title: String,
    pub status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug)]
pub struct ApiError {
    problem: Problem,
}

impl ApiError {
    pub fn new(status: StatusCode, kind: &'static str, title: impl Into<String>) -> Self {
        Self {
            problem: Problem {
                kind,
                title: title.into(),
                status: status.as_u16(),
                detail: None,
            },
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.problem.detail = Some(detail.into());
        self
    }

    pub fn not_found() -> Self {
        Self::new(StatusCode::NOT_FOUND, "not_found", "Not found")
    }

    pub fn unauthorized() -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "Authentication required",
        )
    }

    pub fn forbidden() -> Self {
        Self::new(StatusCode::FORBIDDEN, "forbidden", "Not allowed")
    }

    pub fn bad_request(title: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "bad_request", title)
    }

    pub fn conflict(title: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, "conflict", title)
    }

    pub fn internal() -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "Internal server error",
        )
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let mut response = (
            StatusCode::from_u16(self.problem.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(&self.problem),
        )
            .into_response();
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static(PROBLEM_JSON),
        );
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use http_body_util::BodyExt;

    #[tokio::test]
    async fn problem_carries_type_status_and_content_type() {
        let response = ApiError::forbidden()
            .with_detail("book belongs to another user")
            .into_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(response.headers()[header::CONTENT_TYPE], PROBLEM_JSON);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["type"], "forbidden");
        assert_eq!(json["status"], 403);
        assert_eq!(json["detail"], "book belongs to another user");
    }
}
