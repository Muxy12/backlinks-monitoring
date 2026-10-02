//! Cloudflare Turnstile verification — anti-spam on the public, unauthenticated
//! forms (register, password-reset request, the free backlink checker).
//!
//! Env-gated on `TURNSTILE_SECRET_KEY`. When it is unset the challenge is
//! skipped entirely (verify is a no-op), so local dev, tests and CI run without
//! keys — the same inert-until-configured pattern as Resend and Stripe.
//!
//! siteverify is a fixed, trusted Cloudflare endpoint (not a user-supplied URL),
//! so no SSRF guard is needed — unlike the crawler's outbound fetches.

use crate::error::ApiError;

const SITEVERIFY_URL: &str = "https://challenges.cloudflare.com/turnstile/v0/siteverify";

/// Validate a Turnstile token submitted with a public form.
///
/// - Secret unset → `Ok(())` (feature disabled).
/// - Secret set, token missing/empty → `422` the form can show.
/// - Secret set, Cloudflare says `success: false` → `422`.
/// - Secret set, but Cloudflare is unreachable or returns garbage → `Ok(())`
///   (fail open): a captcha-vendor outage must not take the signup form down.
///   The explicit negative verdict is the only case that blocks.
pub async fn verify(token: Option<&str>) -> Result<(), ApiError> {
    let Some(secret) = std::env::var("TURNSTILE_SECRET_KEY")
        .ok()
        .filter(|s| !s.is_empty())
    else {
        return Ok(()); // not configured → skip
    };

    let token = token
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .ok_or_else(|| ApiError::Validation("please complete the anti-spam challenge".into()))?;

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "turnstile: http client build failed, allowing");
            return Ok(());
        }
    };

    let body = serde_json::json!({ "secret": secret, "response": token });
    let resp = match client.post(SITEVERIFY_URL).json(&body).send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(error = %e, "turnstile: siteverify unreachable, allowing");
            return Ok(());
        }
    };

    let payload = match resp.json::<serde_json::Value>().await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, "turnstile: unreadable siteverify response, allowing");
            return Ok(());
        }
    };

    if payload
        .get("success")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        Ok(())
    } else {
        tracing::info!(codes = ?payload.get("error-codes"), "turnstile challenge rejected");
        Err(ApiError::Validation(
            "anti-spam challenge failed — please try again".into(),
        ))
    }
}
