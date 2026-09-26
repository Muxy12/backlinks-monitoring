//! Auth routes (docs/09 §Auth): register, login, logout. Login/register are
//! deliberately uniform on failure and never reveal whether an email exists
//! (S4). Sessions are opaque cookies; only hashes are stored (S4).

use crate::error::ApiError;
use crate::session::{clear_cookies, current, issue_cookies};
use crate::state::AppState;
use axum::Json;
use axum::extract::State;
use axum_extra::extract::cookie::CookieJar;
use chrono::{Duration, Utc};
use db::repositories::{sessions, users};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct RegisterBody {
    pub email: String,
    pub password: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct UserView {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub email_verified: bool,
    /// Platform operator flag — the frontend uses it to reveal the admin nav.
    pub is_admin: bool,
}

impl From<db::entities::User> for UserView {
    fn from(u: db::entities::User) -> Self {
        UserView {
            id: u.id,
            email: u.email,
            name: u.name,
            email_verified: u.email_verified_at.is_some(),
            is_admin: u.is_platform_admin,
        }
    }
}

fn validate_credentials(email: &str, password: &str) -> Result<(), ApiError> {
    if !email.contains('@') || email.len() > 254 {
        return Err(ApiError::Validation("a valid email is required".into()));
    }
    if password.len() < 8 {
        return Err(ApiError::Validation(
            "password must be at least 8 characters".into(),
        ));
    }
    if password.len() > 1024 {
        return Err(ApiError::Validation("password is too long".into()));
    }
    Ok(())
}

/// POST /v1/auth/register — create an account. Returns the user; does not log
/// them in (email verification would precede first login in a fuller flow).
pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterBody>,
) -> Result<(axum::http::StatusCode, Json<UserView>), ApiError> {
    validate_credentials(&body.email, &body.password)?;
    let name = body.name.trim();
    if name.is_empty() {
        return Err(ApiError::Validation("name is required".into()));
    }

    let hash = auth::hash_password(&body.password, &state.cfg.argon)?;
    let user = users::create(&state.pool, Uuid::now_v7(), body.email.trim(), &hash, name).await?;
    Ok((axum::http::StatusCode::CREATED, Json(user.into())))
}

#[derive(Debug, Deserialize)]
pub struct LoginBody {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub user: UserView,
}

/// POST /v1/auth/login — verify credentials, open a session, set cookies.
pub async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<LoginBody>,
) -> Result<(CookieJar, Json<LoginResponse>), ApiError> {
    // Uniform failure: look the user up, but present "not found" and "bad
    // password" identically (S4).
    let user = users::find_by_email(&state.pool, body.email.trim())
        .await?
        .ok_or(ApiError::Unauthorized)?;
    auth::verify_password(&body.password, &user.password_hash)?;

    let token = auth::token::Token::generate();
    let csrf = auth::token::Token::generate();
    let expires = Utc::now() + Duration::hours(state.cfg.session_ttl_hours);
    sessions::create(
        &state.pool,
        Uuid::now_v7(),
        user.id,
        &token.hash,
        &csrf.hash,
        expires,
    )
    .await?;

    let jar = issue_cookies(
        jar,
        token.secret,
        csrf.secret,
        state.cfg.secure_cookies,
        state.cfg.session_ttl_hours,
    );
    Ok((jar, Json(LoginResponse { user: user.into() })))
}

/// POST /v1/auth/logout — revoke the current session and clear cookies.
pub async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<(CookieJar, axum::http::StatusCode), ApiError> {
    // Best-effort: if there is a valid session, revoke it in the database (not
    // just drop the cookie, docs/09 §Auth).
    if let Ok((_, session)) = current(&state, &jar).await {
        let _ = sessions::revoke(&state.pool, session.id).await;
    }
    Ok((clear_cookies(jar), axum::http::StatusCode::NO_CONTENT))
}
