//! Stripe webhook (§39, docs/11 S5). Unauthenticated but signature-verified on
//! the RAW body, and idempotent: a replayed event is a no-op (200). Full
//! subscription-state mutation from events lands with the live billing service
//! (needs Stripe keys); this endpoint already gives safe, idempotent receipt.

use crate::error::ApiError;
use crate::state::AppState;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};

pub async fn stripe_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, ApiError> {
    let Some(secret) = state.cfg.stripe_webhook_secret.as_deref() else {
        // No secret configured — the endpoint is effectively disabled.
        return Err(ApiError::NotFound);
    };
    let sig = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .ok_or(ApiError::Unauthorized)?;
    let payload =
        std::str::from_utf8(&body).map_err(|_| ApiError::Validation("non-utf8 body".into()))?;

    let now = chrono::Utc::now().timestamp();
    billing::verify_signature(payload, sig, secret, now, 300)
        .map_err(|_| ApiError::Unauthorized)?;

    // Extract id + type from the verified payload.
    let event: serde_json::Value =
        serde_json::from_str(payload).map_err(|_| ApiError::Validation("bad JSON".into()))?;
    let event_id = event
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or(ApiError::Validation("no event id".into()))?;
    let event_type = event
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    // Idempotent receipt: first time -> record (and would process); replay -> 200.
    let _first_time =
        db::repositories::billing::record_stripe_event(&state.pool, event_id, event_type).await?;
    // Subscription-state application from `event` is a follow-up (needs the
    // Stripe price->plan mapping from live keys). Receipt is idempotent already.

    Ok(StatusCode::OK)
}
