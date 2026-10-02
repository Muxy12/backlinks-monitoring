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
    #[error("suspended")]
    Suspended,
    #[error("not found")]
    NotFound,
    #[error("conflict")]
    Conflict(String),
    #[error("plan limit reached")]
    PlanLimit(String),
    #[error("rate limited")]
    RateLimited { retry_after_secs: u64 },
    #[error("service busy")]
    Busy,
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
            ApiError::Suspended => (
                StatusCode::FORBIDDEN,
                "suspended",
                "this organization has been suspended".into(),
            ),
            ApiError::NotFound => (StatusCode::NOT_FOUND, "not_found", "not found".into()),
            ApiError::Conflict(d) => (StatusCode::CONFLICT, "conflict", d.clone()),
            ApiError::PlanLimit(d) => (StatusCode::PAYMENT_REQUIRED, "plan_limit", d.clone()),
            ApiError::RateLimited { .. } => (
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limited",
                "too many requests — slow down".into(),
            ),
            ApiError::Busy => (
                StatusCode::SERVICE_UNAVAILABLE,
                "busy",
                "the service is at capacity — try again shortly".into(),
            ),
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
        let retry_after = match self {
            ApiError::RateLimited { retry_after_secs } => Some(retry_after_secs),
            _ => None,
        };
        let (status, kind, detail) = self.parts();
        let body = Json(json!({
            "type": format!("https://linkpulz.com/errors/{kind}"),
            "title": kind,
            "status": status.as_u16(),
            "detail": detail,
        }));
        let mut res = (status, body).into_response();
        if let Some(secs) = retry_after
            && let Ok(v) = axum::http::HeaderValue::from_str(&secs.to_string())
        {
            res.headers_mut().insert(axum::http::header::RETRY_AFTER, v);
        }
        res
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
            db::DbError::Suspended => ApiError::Suspended,
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
