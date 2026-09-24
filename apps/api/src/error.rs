//! API error type and its RFC 7807-style JSON rendering (docs/09 §1).
//! Cross-tenant access maps to 404, never 403 (ADR-0004).

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("validation failed")]
    Validation(String),
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("not found")]
    NotFound,
    #[error("conflict")]
    Conflict(String),
    #[error("plan limit reached")]
    PlanLimit(String),
    #[error("internal error")]
    Internal,
}

impl ApiError {
    fn parts(&self) -> (StatusCode, &'static str, String) {
        match self {
            ApiError::Validation(d) => (StatusCode::UNPROCESSABLE_ENTITY, "validation", d.clone()),
            ApiError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "authentication required".into(),
            ),
            ApiError::Forbidden => (
                StatusCode::FORBIDDEN,
                "forbidden",
                "insufficient role".into(),
            ),
            ApiError::NotFound => (StatusCode::NOT_FOUND, "not_found", "not found".into()),
            ApiError::Conflict(d) => (StatusCode::CONFLICT, "conflict", d.clone()),
            ApiError::PlanLimit(d) => (StatusCode::PAYMENT_REQUIRED, "plan_limit", d.clone()),
            ApiError::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal",
                "internal error".into(),
            ),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, kind, detail) = self.parts();
        let body = Json(json!({
            "type": format!("https://backlio.app/errors/{kind}"),
            "title": kind,
            "status": status.as_u16(),
            "detail": detail,
        }));
        (status, body).into_response()
    }
}

/// Map database errors to API errors. `NotFound`/`Forbidden` pass through with
/// their status; a unique-violation becomes a 409 naming the field; everything
/// else is a 500 with no internal detail leaked (S10).
impl From<db::DbError> for ApiError {
    fn from(e: db::DbError) -> Self {
        match e {
            db::DbError::NotFound => ApiError::NotFound,
            db::DbError::Forbidden => ApiError::Forbidden,
            db::DbError::Conflict(constraint) => ApiError::Conflict(field_for(&constraint)),
            other => {
                tracing::error!(error = %other, "database error");
                ApiError::Internal
            }
        }
    }
}

/// Turn a Postgres constraint name into a user-facing field hint.
fn field_for(constraint: &str) -> String {
    if constraint.contains("email") {
        "email already in use".into()
    } else if constraint.contains("slug") {
        "that workspace URL is taken".into()
    } else {
        "already exists".into()
    }
}

impl From<auth::AuthError> for ApiError {
    fn from(e: auth::AuthError) -> Self {
        match e {
            // Uniform: never reveal whether the email existed (S4).
            auth::AuthError::InvalidCredentials | auth::AuthError::MalformedHash => {
                ApiError::Unauthorized
            }
            auth::AuthError::HashingFailed => ApiError::Internal,
        }
    }
}
