//! Pre-launch waitlist (public). A visitor on the "coming soon" homepage leaves
//! an email; we store it, deduped. Anti-spam reuses Turnstile (env-gated, like
//! register) and first-touch attribution is captured like signup (docs/20 §P8).

use crate::error::ApiError;
use crate::routes::auth_routes::AttributionBody;
use crate::state::AppState;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct JoinBody {
    pub email: String,
    #[serde(default)]
    pub turnstile_token: Option<String>,
    #[serde(default)]
    pub attribution: Option<AttributionBody>,
}

#[derive(Debug, Serialize)]
pub struct JoinResponse {
    pub ok: bool,
    /// True when the email was already on the list (still a success for the UI).
    pub already: bool,
}

/// Trim, bound and drop-if-empty an attribution value.
fn clip(s: Option<String>) -> Option<String> {
    s.map(|v| v.trim().chars().take(400).collect::<String>())
        .filter(|v| !v.is_empty())
}

/// POST /v1/waitlist — add an email to the pre-launch list. Public, no session.
pub async fn join(
    State(state): State<AppState>,
    Json(body): Json<JoinBody>,
) -> Result<(StatusCode, Json<JoinResponse>), ApiError> {
    crate::turnstile::verify(body.turnstile_token.as_deref()).await?;

    let email = body.email.trim().to_ascii_lowercase();
    if email.len() < 3 || email.len() > 320 || !email.contains('@') {
        return Err(ApiError::Validation("a valid email is required".into()));
    }

    let a = body.attribution.unwrap_or_default();
    let (src, med, camp, refr, land) = (
        clip(a.utm_source),
        clip(a.utm_medium),
        clip(a.utm_campaign),
        clip(a.referrer),
        clip(a.landing_path),
    );
    // Opaque per-signup token for the unsubscribe link.
    let token = auth::token::Token::generate().secret;
    let created = db::repositories::waitlist::add(
        &state.pool,
        db::repositories::waitlist::NewSignup {
            email: &email,
            unsubscribe_token: &token,
            utm_source: src.as_deref(),
            utm_medium: med.as_deref(),
            utm_campaign: camp.as_deref(),
            referrer: refr.as_deref(),
            landing_path: land.as_deref(),
        },
    )
    .await?;

    // Confirmation email, only on a genuinely new signup (best-effort; a missing
    // email provider just no-ops, and a failure never fails the signup). On a
    // successful send we stamp confirmation_sent_at so the admin "resend pending"
    // action skips it; a failed send stays pending for a later retry.
    if created {
        // Store the contact in the Resend audience (mailing list) so the list can
        // be used for the launch broadcast. Best-effort: a failure or missing
        // email config never fails the signup.
        if let Err(e) = notifications::resend::add_to_audience(&email).await {
            tracing::warn!(error = %e, "waitlist contact not added to resend audience");
        }
        match notifications::waitlist::send_confirmation(&email, &token).await {
            Ok(()) => {
                let _ = db::repositories::waitlist::mark_confirmed(&state.pool, &email).await;
            }
            Err(e) => tracing::warn!(error = %e, "waitlist confirmation email not sent"),
        }
    }

    Ok((
        StatusCode::CREATED,
        Json(JoinResponse {
            ok: true,
            already: !created,
        }),
    ))
}

#[derive(Debug, Serialize)]
pub struct CountResponse {
    pub count: i64,
}

/// GET /v1/waitlist/count — public total number of waitlist signups, for the
/// social-proof counter on the pre-launch page. Aggregate only, no emails.
pub async fn count(State(state): State<AppState>) -> Result<Json<CountResponse>, ApiError> {
    let count = db::repositories::waitlist::count(&state.pool).await?;
    Ok(Json(CountResponse { count }))
}

#[derive(Debug, Deserialize)]
pub struct UnsubscribeBody {
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct UnsubscribeResponse {
    pub ok: bool,
    /// Whether a matching signup was found and removed.
    pub removed: bool,
}

/// POST /v1/waitlist/unsubscribe — remove a signup by its unsubscribe token
/// (from the email link). Public: the opaque token is the authorization.
/// Idempotent — an unknown token returns ok with removed=false.
pub async fn unsubscribe(
    State(state): State<AppState>,
    Json(body): Json<UnsubscribeBody>,
) -> Result<Json<UnsubscribeResponse>, ApiError> {
    let token = body.token.trim();
    if token.is_empty() || token.len() > 200 {
        return Err(ApiError::Validation("invalid token".into()));
    }
    let removed_email = db::repositories::waitlist::unsubscribe(&state.pool, token).await?;
    if let Some(email) = &removed_email {
        // Keep the Resend audience in sync. Best-effort; a failure here still
        // leaves the signup removed from our own list.
        if let Err(e) = notifications::resend::remove_from_audience(email).await {
            tracing::warn!(error = %e, "waitlist contact not removed from resend audience");
        }
    }
    Ok(Json(UnsubscribeResponse {
        ok: true,
        removed: removed_email.is_some(),
    }))
}
