//! Newsletter subscription (public). Kept SEPARATE from the waitlist: contacts
//! go to their own Resend audience (`RESEND_NEWSLETTER_AUDIENCE_ID`) so campaigns
//! can target each list independently, and no waitlist confirmation email is
//! sent. Anti-spam reuses Turnstile (env-gated), like the waitlist.

use crate::error::ApiError;
use axum::Json;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct SubscribeBody {
    pub email: String,
    #[serde(default)]
    pub turnstile_token: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SubscribeResponse {
    pub ok: bool,
}

/// POST /v1/newsletter — add an email to the newsletter audience. Public, no
/// session. Email delivery is best-effort: a missing/failing provider never
/// fails the request (the UI has already shown success and the user can retry).
pub async fn subscribe(
    Json(body): Json<SubscribeBody>,
) -> Result<(StatusCode, Json<SubscribeResponse>), ApiError> {
    crate::turnstile::verify(body.turnstile_token.as_deref()).await?;

    let email = body.email.trim().to_ascii_lowercase();
    if email.len() < 3 || email.len() > 320 || !email.contains('@') {
        return Err(ApiError::Validation("a valid email is required".into()));
    }

    if let Err(e) = notifications::resend::add_to_newsletter(&email).await {
        tracing::warn!(error = %e, "newsletter contact not added to resend audience");
    }

    Ok((StatusCode::OK, Json(SubscribeResponse { ok: true })))
}
