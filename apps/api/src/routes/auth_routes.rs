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
use db::repositories::{auth_tokens, sessions, users};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The app origin for emailed links (env `APP_URL`, else the live front).
fn app_url() -> String {
    std::env::var("APP_URL")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "https://web-sooty-one-68.vercel.app".to_string())
        .trim_end_matches('/')
        .to_string()
}

/// Mint a single-use token, persist its hash, and return the secret to embed in
/// a link. `kind` is 'verify_email' | 'password_reset'.
async fn issue_token(
    state: &AppState,
    user_id: Uuid,
    kind: &str,
    ttl: Duration,
) -> Result<String, ApiError> {
    let token = auth::token::Token::generate();
    auth_tokens::create(
        &state.pool,
        Uuid::now_v7(),
        user_id,
        kind,
        &token.hash,
        Utc::now() + ttl,
    )
    .await?;
    Ok(token.secret)
}

/// Send a link email via Resend, best-effort (email being unconfigured must not
/// fail registration/reset). Logs on failure.
async fn send_link_email(to: &str, subject: &str, intro: &str, url: &str) {
    let text =
        format!("{intro}\n\n{url}\n\nIf you didn't request this, ignore this email.\n\n— LinkPulz");
    if let Err(e) = notifications::resend::send_email(to, subject, &text).await {
        tracing::warn!(error = %e, subject, "auth email not sent");
    }
}

#[derive(Debug, Deserialize)]
pub struct RegisterBody {
    pub email: String,
    pub password: String,
    pub name: String,
    /// Cloudflare Turnstile token (anti-spam). Optional: only enforced when
    /// `TURNSTILE_SECRET_KEY` is set on the server.
    #[serde(default)]
    pub turnstile_token: Option<String>,
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
    crate::turnstile::verify(body.turnstile_token.as_deref()).await?;
    validate_credentials(&body.email, &body.password)?;
    let name = body.name.trim();
    if name.is_empty() {
        return Err(ApiError::Validation("name is required".into()));
    }

    let hash = auth::hash_password(&body.password, &state.cfg.argon)?;
    let user = users::create(&state.pool, Uuid::now_v7(), body.email.trim(), &hash, name).await?;

    // Send a verification link (best-effort; account is usable meanwhile).
    if let Ok(secret) =
        issue_token(&state, user.id, "email_verification", Duration::hours(48)).await
    {
        let url = format!("{}/verify?token={}", app_url(), secret);
        send_link_email(
            &user.email,
            "Verify your LinkPulz email",
            "Confirm your email address to secure your LinkPulz account:",
            &url,
        )
        .await;
    }
    Ok((axum::http::StatusCode::CREATED, Json(user.into())))
}

#[derive(Debug, Deserialize)]
pub struct TokenBody {
    pub token: String,
}

/// POST /v1/auth/verify — consume a verification token and mark the email verified.
pub async fn verify_email(
    State(state): State<AppState>,
    Json(body): Json<TokenBody>,
) -> Result<axum::http::StatusCode, ApiError> {
    let hash = auth::hash_token(body.token.trim());
    let user_id = auth_tokens::consume(&state.pool, "email_verification", &hash)
        .await?
        .ok_or_else(|| ApiError::Validation("invalid or expired verification link".into()))?;
    users::mark_email_verified(&state.pool, user_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct EmailBody {
    pub email: String,
    /// Cloudflare Turnstile token (anti-spam). Enforced on `/forgot` when
    /// `TURNSTILE_SECRET_KEY` is set; ignored by `/resend-verification`.
    #[serde(default)]
    pub turnstile_token: Option<String>,
}

/// POST /v1/auth/resend-verification — re-send the verification email. Always
/// 204 (never reveals whether the email exists or is already verified, S4).
pub async fn resend_verification(
    State(state): State<AppState>,
    Json(body): Json<EmailBody>,
) -> Result<axum::http::StatusCode, ApiError> {
    if let Ok(Some(user)) = users::find_by_email(&state.pool, body.email.trim()).await
        && user.email_verified_at.is_none()
        && let Ok(secret) =
            issue_token(&state, user.id, "email_verification", Duration::hours(48)).await
    {
        let url = format!("{}/verify?token={}", app_url(), secret);
        send_link_email(
            &user.email,
            "Verify your LinkPulz email",
            "Confirm your email address to secure your LinkPulz account:",
            &url,
        )
        .await;
    }
    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// POST /v1/auth/forgot — email a password-reset link. Always 204 (S4).
pub async fn forgot_password(
    State(state): State<AppState>,
    Json(body): Json<EmailBody>,
) -> Result<axum::http::StatusCode, ApiError> {
    crate::turnstile::verify(body.turnstile_token.as_deref()).await?;
    if let Ok(Some(user)) = users::find_by_email(&state.pool, body.email.trim()).await
        && let Ok(secret) = issue_token(&state, user.id, "password_reset", Duration::hours(1)).await
    {
        let url = format!("{}/reset?token={}", app_url(), secret);
        send_link_email(
            &user.email,
            "Reset your LinkPulz password",
            "Use this link to set a new password (valid for 1 hour):",
            &url,
        )
        .await;
    }
    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct ResetBody {
    pub token: String,
    pub password: String,
}

/// POST /v1/auth/reset — set a new password from a reset token, then revoke all
/// the user's existing sessions.
pub async fn reset_password(
    State(state): State<AppState>,
    Json(body): Json<ResetBody>,
) -> Result<axum::http::StatusCode, ApiError> {
    if body.password.len() < 8 || body.password.len() > 1024 {
        return Err(ApiError::Validation(
            "password must be 8–1024 characters".into(),
        ));
    }
    let hash = auth::hash_token(body.token.trim());
    let user_id = auth_tokens::consume(&state.pool, "password_reset", &hash)
        .await?
        .ok_or_else(|| ApiError::Validation("invalid or expired reset link".into()))?;
    let new_hash = auth::hash_password(&body.password, &state.cfg.argon)?;
    users::set_password(&state.pool, user_id, &new_hash).await?;
    let _ = sessions::revoke_all_for_user(&state.pool, user_id).await;
    Ok(axum::http::StatusCode::NO_CONTENT)
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
