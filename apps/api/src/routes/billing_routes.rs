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
    /// When the free trial ends (RFC3339), only while the org is `trialing`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trial_ends_at: Option<String>,
    /// Whole days left in the trial (>= 0), only while trialing and not expired.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trial_days_left: Option<i64>,
    /// Whether the org may add/monitor right now (active/past_due, or a live
    /// trial). False means the trial expired or the subscription is inactive.
    pub access_active: bool,
}

/// GET current plan/status for the org (+ trial state + whether Stripe is wired).
pub async fn billing_status(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
) -> Result<Json<BillingStatus>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Viewer)?;
    let (plan, status, trial_end) =
        db::repositories::billing::status_for_org(&state.pool, org_id).await?;

    let now = chrono::Utc::now();
    let trialing_live = status == "trialing" && trial_end.map(|e| e > now).unwrap_or(false);
    let (trial_ends_at, trial_days_left) = if trialing_live {
        let end = trial_end.expect("live trial has an end");
        // Round up: ~1 hour left still reads as "1 day left", not "0".
        let days = (((end - now).num_seconds().max(0)) as f64 / 86_400.0).ceil() as i64;
        (Some(end.to_rfc3339()), Some(days))
    } else {
        (None, None)
    };
    let access_active = matches!(status.as_str(), "active" | "past_due") || trialing_live;

    Ok(Json(BillingStatus {
        plan,
        status,
        configured: billing::stripe::is_configured(),
        trial_ends_at,
        trial_days_left,
        access_active,
    }))
}

/// One usage dimension: how many are in use against the plan's cap (`None` =
/// unlimited).
#[derive(Serialize)]
pub struct UsageItem {
    pub used: i64,
    pub limit: Option<i64>,
}

/// GET current usage vs. plan limits for the org (billing page meters).
#[derive(Serialize)]
pub struct UsageView {
    pub plan: String,
    pub backlinks: UsageItem,
    pub projects: UsageItem,
    pub seats: UsageItem,
}

pub async fn billing_usage(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
) -> Result<Json<UsageView>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Viewer)?;

    let spec = db::repositories::billing::plan_for_org(&state.pool, org_id)
        .await?
        .spec();
    let backlinks = db::repositories::backlinks::count_active(&state.pool, &scope).await?;
    let projects = db::repositories::projects::count(&state.pool, &scope).await?;
    let seats = db::repositories::members::count_for_org(&state.pool, org_id).await?;

    Ok(Json(UsageView {
        plan: spec.slug.to_string(),
        backlinks: UsageItem {
            used: backlinks,
            limit: spec.backlink_limit.map(|v| v as i64),
        },
        projects: UsageItem {
            used: projects,
            limit: spec.project_limit.map(|v| v as i64),
        },
        seats: UsageItem {
            used: seats,
            limit: spec.seat_limit.map(|v| v as i64),
        },
    }))
}

#[derive(Deserialize)]
pub struct CheckoutBody {
    pub plan: String,
    /// "monthly" (default) or "annual". Anything else is treated as monthly.
    #[serde(default)]
    pub interval: Option<String>,
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
    let interval = billing::stripe::Interval::from_opt(body.interval.as_deref());
    let price = billing::stripe::price_for_plan_interval(plan, interval).ok_or_else(|| {
        let suffix = match interval {
            billing::stripe::Interval::Annual => " (annual)",
            billing::stripe::Interval::Monthly => "",
        };
        ApiError::Validation(format!(
            "no Stripe price configured for the {} plan{suffix}",
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
        state.cfg.stripe_tax_enabled,
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

/// A Unix-seconds field on a Stripe object → UTC timestamp (None if absent/null).
fn ts(obj: &serde_json::Value, key: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    obj.get(key)
        .and_then(|v| v.as_i64())
        .and_then(|s| chrono::DateTime::from_timestamp(s, 0))
}

/// The subscription id an invoice belongs to, across Stripe API shapes.
fn invoice_subscription_id(obj: &serde_json::Value) -> Option<&str> {
    obj["subscription"]
        .as_str()
        .or_else(|| obj["parent"]["subscription_details"]["subscription"].as_str())
}

/// Apply a verified Stripe event to our subscription state (docs/20 — P4).
async fn apply_event(
    state: &AppState,
    event_type: &str,
    obj: &serde_json::Value,
) -> Result<(), ApiError> {
    match event_type {
        // The row is first created here; the subscription.* events then enrich it
        // with period/trial/cancel data. Keep this light so a later-arriving
        // subscription event is the source of truth for those fields.
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
                // Funnel: trial/visitor converted to a paid subscription (docs/20 §P8).
                let _ = db::repositories::analytics::record(
                    &state.pool,
                    "subscribed",
                    None,
                    Some(org),
                    None,
                    serde_json::json!({ "plan": plan }),
                )
                .await;
            }
        }
        // Source of truth for plan, status and period/trial/cancel. Keyed by org
        // (from subscription metadata) so it is robust to event ordering; falls
        // back to a status update by subscription id when metadata is absent.
        "customer.subscription.updated" | "customer.subscription.created" => {
            let status = obj["status"].as_str().unwrap_or("active");
            let sub_id = obj["id"].as_str();
            let plan = obj["items"]["data"][0]["price"]["id"]
                .as_str()
                .and_then(billing::stripe::plan_from_price)
                .map(|p| p.spec().slug);
            let org_id = obj["metadata"]["organization_id"]
                .as_str()
                .and_then(|s| Uuid::parse_str(s).ok());
            let customer = obj["customer"].as_str();

            match (org_id, customer) {
                (Some(org), Some(cust)) => {
                    db::repositories::billing::upsert_subscription_from_stripe(
                        &state.pool,
                        org,
                        cust,
                        sub_id,
                        plan,
                        status,
                        ts(obj, "current_period_start"),
                        ts(obj, "current_period_end"),
                        obj["cancel_at_period_end"].as_bool().unwrap_or(false),
                        ts(obj, "canceled_at"),
                        ts(obj, "trial_end"),
                    )
                    .await?;
                }
                _ => {
                    if let Some(sid) = sub_id {
                        db::repositories::billing::update_subscription_by_stripe_id(
                            &state.pool,
                            sid,
                            plan,
                            status,
                        )
                        .await?;
                    }
                }
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
        // Dunning (past-due grace). A failed payment moves a live subscription to
        // past_due — which still grants access — until Stripe recovers it or
        // eventually cancels it. A succeeded payment brings past_due back to active.
        "invoice.payment_failed" => {
            if let Some(sub_id) = invoice_subscription_id(obj) {
                db::repositories::billing::mark_past_due(&state.pool, sub_id).await?;
            }
        }
        "invoice.payment_succeeded" | "invoice.paid" => {
            if let Some(sub_id) = invoice_subscription_id(obj) {
                db::repositories::billing::mark_active_if_recoverable(&state.pool, sub_id).await?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// A plan as served to the pricing / billing UI — straight from the catalog so
/// the client never hardcodes prices or limits (§39, one source of truth).
#[derive(Serialize)]
pub struct PlanView {
    pub slug: String,
    pub name: String,
    pub price_monthly_cents: u32,
    pub price_annual_cents: u32,
    /// `null` = unlimited.
    pub backlinks: Option<u32>,
    pub projects: Option<u32>,
    pub seats: Option<u32>,
    pub monitoring_interval_hours: u32,
    pub most_popular: bool,
}

/// GET /v1/plans — the public plan catalog. No auth: it drives the marketing
/// pricing page and the in-app billing page from one server-side source.
pub async fn list_plans() -> Json<Vec<PlanView>> {
    Json(
        Plan::ALL
            .into_iter()
            .map(|p| {
                let s = p.spec();
                PlanView {
                    slug: s.slug.to_string(),
                    name: s.display_name.to_string(),
                    price_monthly_cents: s.price_cents,
                    price_annual_cents: s.annual_price_cents,
                    backlinks: s.backlink_limit,
                    projects: s.project_limit,
                    seats: s.seat_limit,
                    monitoring_interval_hours: s.monitoring_interval_hours,
                    most_popular: s.most_popular,
                }
            })
            .collect(),
    )
}
