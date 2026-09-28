//! Billing (§39): Stripe Checkout + Customer Portal + the signature-verified,
//! idempotent webhook that applies subscription state. All Stripe calls are gated
//! on `STRIPE_SECRET_KEY` + per-plan `STRIPE_PRICE_*`; absent, the endpoints
//! return a clear "not configured" error rather than pretending.

use crate::error::ApiError;
use crate::session::{check_csrf, current};
use crate::state::AppState;
use axum::Json;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum_extra::extract::cookie::CookieJar;
use billing::Plan;
use common::Role;
use db::authorize_org;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The app origin for Checkout return URLs (env `APP_URL`, else the live front).
fn app_url() -> String {
    std::env::var("APP_URL")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "https://web-sooty-one-68.vercel.app".to_string())
        .trim_end_matches('/')
        .to_string()
}

#[derive(Serialize)]
pub struct BillingStatus {
    pub plan: String,
    pub status: String,
    /// Whether Stripe is configured on the server (drives the UI's buttons).
    pub configured: bool,
}

/// GET current plan/status for the org (+ whether Stripe is wired).
pub async fn billing_status(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
) -> Result<Json<BillingStatus>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Viewer)?;
    let (plan, status) = db::repositories::billing::status_for_org(&state.pool, org_id).await?;
    Ok(Json(BillingStatus {
        plan,
        status,
        configured: billing::stripe::is_configured(),
    }))
}

#[derive(Deserialize)]
pub struct CheckoutBody {
    pub plan: String,
}

#[derive(Serialize)]
pub struct UrlResponse {
    pub url: String,
}

/// POST — start a Checkout Session for `plan`; returns the hosted Stripe URL.
pub async fn create_checkout(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<CheckoutBody>,
) -> Result<Json<UrlResponse>, ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Owner)?; // only an owner manages billing

    let plan = Plan::from_slug(&body.plan)
        .ok_or_else(|| ApiError::Validation(format!("unknown plan '{}'", body.plan)))?;
    let secret = billing::stripe::secret_key()
        .ok_or_else(|| ApiError::Validation("billing is not configured on the server".into()))?;
    let price = billing::stripe::price_for_plan(plan).ok_or_else(|| {
        ApiError::Validation(format!(
            "no Stripe price configured for the {} plan",
            plan.spec().slug
        ))
    })?;

    let base = app_url();
    let success = format!("{base}/settings/billing?checkout=success");
    let cancel = format!("{base}/settings/billing?checkout=cancel");
    let url = billing::stripe::create_checkout_session(
        &secret,
        &price,
        plan.spec().slug,
        &org_id.to_string(),
        &user.email,
        &success,
        &cancel,
    )
    .await
    .map_err(|e| {
        tracing::warn!(error = %e, "stripe checkout failed");
        ApiError::Validation(format!("checkout failed: {e}"))
    })?;
    Ok(Json(UrlResponse { url }))
}

/// POST — open the Customer Portal for the org's Stripe customer.
pub async fn create_portal(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
    headers: HeaderMap,
) -> Result<Json<UrlResponse>, ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Owner)?;

    let secret = billing::stripe::secret_key()
        .ok_or_else(|| ApiError::Validation("billing is not configured on the server".into()))?;
    let customer = db::repositories::billing::customer_for_org(&state.pool, org_id)
        .await?
        .ok_or_else(|| ApiError::Validation("no subscription yet — subscribe first".into()))?;
    let url = billing::stripe::create_portal_session(
        &secret,
        &customer,
        &format!("{}/settings/billing", app_url()),
    )
    .await
    .map_err(|e| {
        tracing::warn!(error = %e, "stripe portal failed");
        ApiError::Validation(format!("portal failed: {e}"))
    })?;
    Ok(Json(UrlResponse { url }))
}

pub async fn stripe_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, ApiError> {
    let Some(secret) = state.cfg.stripe_webhook_secret.as_deref() else {
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

    // Idempotent: only the first delivery of an event id applies state.
    let first_time =
        db::repositories::billing::record_stripe_event(&state.pool, event_id, event_type).await?;
    // Still 200 even on apply error: Stripe would retry, and our receipt is
    // recorded — we log rather than loop forever on a malformed event.
    if first_time && let Err(e) = apply_event(&state, event_type, &event["data"]["object"]).await {
        tracing::error!(event_type, error = %e, "failed to apply stripe event");
    }
    Ok(StatusCode::OK)
}

/// Apply a verified Stripe event to our subscription state.
async fn apply_event(
    state: &AppState,
    event_type: &str,
    obj: &serde_json::Value,
) -> Result<(), ApiError> {
    match event_type {
        "checkout.session.completed" => {
            let org_id = obj["metadata"]["organization_id"]
                .as_str()
                .or_else(|| obj["client_reference_id"].as_str())
                .and_then(|s| Uuid::parse_str(s).ok());
            let customer = obj["customer"].as_str();
            let subscription = obj["subscription"].as_str();
            let plan = obj["metadata"]["plan"].as_str().unwrap_or("starter");
            if let (Some(org), Some(cust)) = (org_id, customer) {
                db::repositories::billing::upsert_subscription(
                    &state.pool,
                    org,
                    cust,
                    subscription,
                    plan,
                    "active",
                )
                .await?;
            }
        }
        "customer.subscription.updated" | "customer.subscription.created" => {
            if let Some(sub_id) = obj["id"].as_str() {
                let status = obj["status"].as_str().unwrap_or("active");
                let price_id = obj["items"]["data"][0]["price"]["id"].as_str();
                let plan = price_id
                    .and_then(billing::stripe::plan_from_price)
                    .map(|p| p.spec().slug);
                db::repositories::billing::update_subscription_by_stripe_id(
                    &state.pool,
                    sub_id,
                    plan,
                    status,
                )
                .await?;
            }
        }
        "customer.subscription.deleted" => {
            if let Some(sub_id) = obj["id"].as_str() {
                db::repositories::billing::update_subscription_by_stripe_id(
                    &state.pool,
                    sub_id,
                    None,
                    "canceled",
                )
                .await?;
            }
        }
        _ => {}
    }
    Ok(())
}
