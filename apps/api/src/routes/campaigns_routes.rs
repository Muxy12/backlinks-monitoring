//! Link-building campaigns (BETA, migration 0036). The goal-driven workflow on
//! top of the existing `campaigns` entity: target pages, a prospect pipeline fed
//! from the competitor-gap analysis, explainable scoring, templated outreach, and
//! a one-click convert that hands a won prospect to the monitoring engine as a
//! protected backlink.
//!
//! Every handler is gated on the `campaigns_beta` flag: when it is off the route
//! 404s (`require_beta`), so the surface is inert unless the operator enables it.
//! Tenant scoping, CSRF on writes, and `Role` checks match the rest of the API.

use crate::error::ApiError;
use crate::session::{check_csrf, current};
use crate::state::AppState;
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum_extra::extract::cookie::CookieJar;
use backlio_core::{
    OutreachContext, ProspectSignals, ReclaimAngle, ReclaimPitch, outreach_render_draft,
    score_prospect,
};
use common::Role;
use db::authorize_org;
use db::repositories::{
    analytics, audit, backlinks, campaign_targets, campaigns, outreach, prospects,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

const STAGES: &[&str] = &[
    "identified",
    "qualified",
    "contacted",
    "negotiating",
    "won",
    "lost",
];
const REL_TYPES: &[&str] = &["dofollow", "nofollow", "sponsored", "ugc"];

/// Inert unless the BETA flag is on: a disabled feature 404s like a missing route.
fn require_beta(state: &AppState) -> Result<(), ApiError> {
    if state.cfg.campaigns_beta {
        Ok(())
    } else {
        Err(ApiError::NotFound)
    }
}

/// Validate + normalize a 3-letter currency code.
fn norm_currency(c: &str) -> Result<String, ApiError> {
    let u = c.trim().to_uppercase();
    if u.len() == 3 && u.chars().all(|ch| ch.is_ascii_alphabetic()) {
        Ok(u)
    } else {
        Err(ApiError::Validation(
            "currency must be a 3-letter code".into(),
        ))
    }
}

/// Format a non-negative money amount as fixed-2 text (DB numeric).
fn money(v: f64) -> Result<String, ApiError> {
    if v < 0.0 {
        return Err(ApiError::Validation("amount cannot be negative".into()));
    }
    Ok(format!("{v:.2}"))
}

fn is_http(s: &str) -> bool {
    let s = s.trim();
    s.starts_with("http://") || s.starts_with("https://")
}

/// Compute the explainable score for a prospect from its local signals.
fn score_for(
    competitor_count: Option<i32>,
    has_contact: bool,
    has_cost: bool,
    stage: &str,
) -> (i32, String) {
    let s = score_prospect(ProspectSignals {
        competitor_count: competitor_count.unwrap_or(0),
        has_contact,
        has_estimated_cost: has_cost,
        stage,
    });
    (s.score as i32, s.reasons.join(", "))
}

// ─── Campaign targets ─────────────────────────────────────────────────────────
#[derive(Serialize)]
pub struct CampaignTargetView {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub url: String,
    pub label: Option<String>,
}

impl From<db::entities::CampaignTarget> for CampaignTargetView {
    fn from(t: db::entities::CampaignTarget) -> Self {
        CampaignTargetView {
            id: t.id,
            campaign_id: t.campaign_id,
            url: t.url,
            label: t.label,
        }
    }
}

pub async fn list_targets(
    State(state): State<AppState>,
    Path((org_id, campaign_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
) -> Result<Json<Vec<CampaignTargetView>>, ApiError> {
    require_beta(&state)?;
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    campaigns::get(&state.pool, &scope, campaign_id).await?; // 404 if not ours
    let rows = campaign_targets::list(&state.pool, &scope, campaign_id).await?;
    Ok(Json(
        rows.into_iter().map(CampaignTargetView::from).collect(),
    ))
}

#[derive(Deserialize)]
pub struct CreateTargetBody {
    pub url: String,
    pub label: Option<String>,
}

pub async fn create_target(
    State(state): State<AppState>,
    Path((org_id, campaign_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<CreateTargetBody>,
) -> Result<(StatusCode, Json<CampaignTargetView>), ApiError> {
    require_beta(&state)?;
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    campaigns::get(&state.pool, &scope, campaign_id).await?;
    let url = body.url.trim();
    if !is_http(url) {
        return Err(ApiError::Validation(
            "url must start with http:// or https://".into(),
        ));
    }
    let label = body
        .label
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let t = campaign_targets::create(&state.pool, &scope, Uuid::now_v7(), campaign_id, url, label)
        .await?;
    Ok((StatusCode::CREATED, Json(t.into())))
}

pub async fn delete_target(
    State(state): State<AppState>,
    Path((org_id, target_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    require_beta(&state)?;
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    if campaign_targets::delete(&state.pool, &scope, target_id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound)
    }
}

// ─── Prospects ────────────────────────────────────────────────────────────────
#[derive(Serialize)]
pub struct ProspectView {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub domain: String,
    pub stage: String,
    pub source: String,
    pub score: Option<i32>,
    pub score_reason: Option<String>,
    pub competitor_count: Option<i32>,
    pub contact_name: Option<String>,
    pub contact_email: Option<String>,
    pub target_url: Option<String>,
    pub estimated_cost: Option<String>,
    pub currency: Option<String>,
    pub notes: Option<String>,
    pub last_contacted_at: Option<String>,
    pub won_backlink_id: Option<Uuid>,
}

impl From<db::entities::Prospect> for ProspectView {
    fn from(p: db::entities::Prospect) -> Self {
        ProspectView {
            id: p.id,
            campaign_id: p.campaign_id,
            domain: p.domain,
            stage: p.stage,
            source: p.source,
            score: p.score,
            score_reason: p.score_reason,
            competitor_count: p.competitor_count,
            contact_name: p.contact_name,
            contact_email: p.contact_email,
            target_url: p.target_url,
            estimated_cost: p.estimated_cost,
            currency: p.currency,
            notes: p.notes,
            last_contacted_at: p.last_contacted_at.map(|d| d.to_rfc3339()),
            won_backlink_id: p.won_backlink_id,
        }
    }
}

#[derive(Deserialize)]
pub struct ListProspectsQuery {
    pub stage: Option<String>,
}

pub async fn list_prospects(
    State(state): State<AppState>,
    Path((org_id, campaign_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    Query(q): Query<ListProspectsQuery>,
) -> Result<Json<Vec<ProspectView>>, ApiError> {
    require_beta(&state)?;
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    campaigns::get(&state.pool, &scope, campaign_id).await?;
    let stage = match q.stage.as_deref() {
        Some(st) if !STAGES.contains(&st) => {
            return Err(ApiError::Validation(format!("unknown stage {st:?}")));
        }
        other => other,
    };
    let rows = prospects::list(&state.pool, &scope, campaign_id, stage).await?;
    Ok(Json(rows.into_iter().map(ProspectView::from).collect()))
}

#[derive(Deserialize)]
pub struct CreateProspectBody {
    pub domain: String,
    pub contact_name: Option<String>,
    pub contact_email: Option<String>,
    pub target_url: Option<String>,
    pub estimated_cost: Option<f64>,
    pub currency: Option<String>,
    pub notes: Option<String>,
}

pub async fn create_prospect(
    State(state): State<AppState>,
    Path((org_id, campaign_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<CreateProspectBody>,
) -> Result<(StatusCode, Json<ProspectView>), ApiError> {
    require_beta(&state)?;
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    campaigns::get(&state.pool, &scope, campaign_id).await?;

    let domain = body.domain.trim();
    if domain.is_empty() || domain.len() > 253 {
        return Err(ApiError::Validation("domain is required".into()));
    }
    let contact_name = body
        .contact_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let contact_email = body
        .contact_email
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let target_url = body
        .target_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let notes = body
        .notes
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let (cost, currency) = match (body.estimated_cost, body.currency.as_deref()) {
        (Some(_), None) => {
            return Err(ApiError::Validation(
                "currency is required when estimated_cost is set".into(),
            ));
        }
        (Some(c), Some(cur)) => (Some(money(c)?), Some(norm_currency(cur)?)),
        (None, _) => (None, None),
    };
    let has_contact = contact_name.is_some() || contact_email.is_some();
    let (score, reason) = score_for(None, has_contact, cost.is_some(), "identified");

    let p = prospects::create(
        &state.pool,
        &scope,
        Uuid::now_v7(),
        prospects::NewProspect {
            campaign_id,
            domain,
            source: "manual",
            contact_name,
            contact_email,
            target_url,
            estimated_cost: cost.as_deref(),
            currency: currency.as_deref(),
            notes,
            competitor_count: None,
            score,
            score_reason: &reason,
        },
    )
    .await?;
    Ok((StatusCode::CREATED, Json(p.into())))
}

#[derive(Deserialize)]
pub struct UpdateProspectBody {
    pub stage: Option<String>,
    pub contact_name: Option<String>,
    pub contact_email: Option<String>,
    pub target_url: Option<String>,
    pub estimated_cost: Option<f64>,
    pub currency: Option<String>,
    pub notes: Option<String>,
}

pub async fn update_prospect(
    State(state): State<AppState>,
    Path((org_id, prospect_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<UpdateProspectBody>,
) -> Result<Json<ProspectView>, ApiError> {
    require_beta(&state)?;
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    let cur = prospects::get(&state.pool, &scope, prospect_id).await?;

    // Merge each field: a provided value wins; absent keeps the current value
    // (BETA PATCH cannot clear a field, matching the backlink edit pattern).
    let merge = |provided: Option<&str>, current: Option<String>| -> Option<String> {
        match provided.map(str::trim) {
            Some(t) if !t.is_empty() => Some(t.to_string()),
            _ => current,
        }
    };
    let stage = match body.stage.as_deref() {
        Some(st) if !STAGES.contains(&st) => {
            return Err(ApiError::Validation(format!("unknown stage {st:?}")));
        }
        Some(st) => st.to_string(),
        None => cur.stage.clone(),
    };
    let contact_name = merge(body.contact_name.as_deref(), cur.contact_name.clone());
    let contact_email = merge(body.contact_email.as_deref(), cur.contact_email.clone());
    let target_url = merge(body.target_url.as_deref(), cur.target_url.clone());
    let notes = merge(body.notes.as_deref(), cur.notes.clone());
    let (estimated_cost, currency) = match body.estimated_cost {
        Some(c) => {
            let cur_code = body
                .currency
                .as_deref()
                .or(cur.currency.as_deref())
                .ok_or_else(|| {
                    ApiError::Validation("currency is required when estimated_cost is set".into())
                })?;
            (Some(money(c)?), Some(norm_currency(cur_code)?))
        }
        None => (
            cur.estimated_cost.clone(),
            match body.currency.as_deref() {
                Some(c) => Some(norm_currency(c)?),
                None => cur.currency.clone(),
            },
        ),
    };

    // Stamp first-contact time when we move into `contacted`.
    let last_contacted_at = if stage == "contacted" && cur.stage != "contacted" {
        Some(chrono::Utc::now())
    } else {
        cur.last_contacted_at
    };

    let has_contact = contact_name.is_some() || contact_email.is_some();
    let (score, reason) = score_for(
        cur.competitor_count,
        has_contact,
        estimated_cost.is_some(),
        &stage,
    );

    let p = prospects::update(
        &state.pool,
        &scope,
        prospect_id,
        prospects::ProspectUpdate {
            stage: &stage,
            contact_name: contact_name.as_deref(),
            contact_email: contact_email.as_deref(),
            target_url: target_url.as_deref(),
            estimated_cost: estimated_cost.as_deref(),
            currency: currency.as_deref(),
            notes: notes.as_deref(),
            score,
            score_reason: &reason,
            last_contacted_at,
            won_backlink_id: cur.won_backlink_id,
        },
    )
    .await?;
    Ok(Json(p.into()))
}

pub async fn delete_prospect(
    State(state): State<AppState>,
    Path((org_id, prospect_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    require_beta(&state)?;
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    if prospects::delete(&state.pool, &scope, prospect_id).await? {
        let _ = audit::log(
            &state.pool,
            Some(org_id),
            Some(user.id),
            Some(user.email.as_str()),
            "prospect_deleted",
            Some("prospect"),
            Some(prospect_id),
            None,
            None,
        )
        .await;
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound)
    }
}

#[derive(Deserialize)]
pub struct BulkStageBody {
    pub ids: Vec<Uuid>,
    pub stage: String,
}

#[derive(Serialize)]
pub struct BulkResult {
    pub updated: u64,
}

pub async fn bulk_stage(
    State(state): State<AppState>,
    Path((org_id, campaign_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<BulkStageBody>,
) -> Result<Json<BulkResult>, ApiError> {
    require_beta(&state)?;
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    campaigns::get(&state.pool, &scope, campaign_id).await?;
    if !STAGES.contains(&body.stage.as_str()) {
        return Err(ApiError::Validation(format!(
            "unknown stage {:?}",
            body.stage
        )));
    }
    if body.ids.is_empty() || body.ids.len() > 5_000 {
        return Err(ApiError::Validation("ids must be 1–5000 items".into()));
    }
    let updated = prospects::bulk_set_stage(&state.pool, &scope, &body.ids, &body.stage).await?;
    Ok(Json(BulkResult { updated }))
}

#[derive(Deserialize)]
pub struct SeedGapBody {
    pub min: Option<i64>,
}

#[derive(Serialize)]
pub struct SeedResult {
    pub created: u64,
}

pub async fn seed_from_gap(
    State(state): State<AppState>,
    Path((org_id, campaign_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<SeedGapBody>,
) -> Result<Json<SeedResult>, ApiError> {
    require_beta(&state)?;
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    let campaign = campaigns::get(&state.pool, &scope, campaign_id).await?;
    let created = prospects::seed_from_gap(
        &state.pool,
        &scope,
        campaign_id,
        campaign.project_id,
        body.min.unwrap_or(1),
        500,
    )
    .await?;
    Ok(Json(SeedResult { created }))
}

// ─── Outreach draft ───────────────────────────────────────────────────────────
#[derive(Deserialize)]
pub struct DraftBody {
    pub sender_name: Option<String>,
}

#[derive(Serialize)]
pub struct DraftView {
    pub subject: String,
    pub body: String,
}

pub async fn prospect_draft(
    State(state): State<AppState>,
    Path((org_id, prospect_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    Json(body): Json<DraftBody>,
) -> Result<Json<DraftView>, ApiError> {
    require_beta(&state)?;
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    let p = prospects::get(&state.pool, &scope, prospect_id).await?;
    let campaign = campaigns::get(&state.pool, &scope, p.campaign_id).await?;
    let target_url = p
        .target_url
        .as_deref()
        .or(campaign.primary_target_url.as_deref());
    let sender_name = body
        .sender_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    // Reclamation prospects (R2) pitch the specific lost link: load the origin
    // backlink and build the receipts. A missing/changed backlink just falls back
    // to the cold template (reclaim stays None).
    let reclaim_bl = match (p.source.as_str(), p.origin_backlink_id) {
        ("reclamation", Some(bl_id)) => backlinks::get(&state.pool, &scope, bl_id).await.ok(),
        _ => None,
    };
    let lost_on = reclaim_bl
        .as_ref()
        .and_then(|b| b.lost_at)
        .map(|d| d.format("%Y-%m-%d").to_string());
    let reclaim = reclaim_bl.as_ref().and_then(|b| {
        ReclaimAngle::from_status(&b.status).map(|angle| ReclaimPitch {
            angle,
            source_url: b.source_url.as_str(),
            anchor: b
                .detected_anchor_text
                .as_deref()
                .or(b.expected_anchor_text.as_deref()),
            lost_on: lost_on.as_deref(),
        })
    });

    let draft = outreach_render_draft(OutreachContext {
        prospect_domain: &p.domain,
        contact_name: p.contact_name.as_deref(),
        target_url,
        sender_name,
        campaign_name: &campaign.name,
        reclaim,
    });
    Ok(Json(DraftView {
        subject: draft.subject,
        body: draft.body,
    }))
}

// ─── Outreach sequence: enroll ("Start sequence") ─────────────────────────────
#[derive(Serialize)]
pub struct StartOutreachResult {
    pub enrolled: bool,
    pub steps_created: u64,
}

/// Enroll a prospect into the automated follow-up sequence: move it to
/// `contacted` (anchoring the cadence) and pre-schedule the steps. Explicit,
/// operator-triggered — nothing sends until this is called (and the server has
/// outreach configured). Idempotent: re-calling never resets the clock or
/// duplicates steps.
pub async fn start_outreach(
    State(state): State<AppState>,
    Path((org_id, prospect_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
) -> Result<Json<StartOutreachResult>, ApiError> {
    require_beta(&state)?;
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    if !notifications::resend::outreach_configured() {
        return Err(ApiError::Validation(
            "outreach sending is not configured on this server".into(),
        ));
    }
    let p = prospects::get(&state.pool, &scope, prospect_id).await?;
    let to_email = p
        .contact_email
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ApiError::Validation("prospect has no contact email".into()))?;
    let now = chrono::Utc::now();
    let enrolled_at =
        outreach::mark_contacted(&state.pool, scope.organization_id(), prospect_id, now)
            .await?
            .ok_or_else(|| ApiError::Validation("prospect is closed (won or lost)".into()))?;
    let steps_created = outreach::enroll(
        &state.pool,
        scope.organization_id(),
        p.campaign_id,
        prospect_id,
        to_email,
        enrolled_at,
    )
    .await?;
    Ok(Json(StartOutreachResult {
        enrolled: true,
        steps_created,
    }))
}

// ─── Outreach unsubscribe (public, no auth) ───────────────────────────────────
#[derive(Deserialize)]
pub struct UnsubQuery {
    pub m: Uuid,
}

/// One-click unsubscribe from an outreach email. Public (the recipient isn't a
/// user). Looks the message up by id, suppresses that recipient for the org, and
/// always returns the same confirmation (never leaks whether the id existed).
pub async fn outreach_unsubscribe(
    State(state): State<AppState>,
    Query(q): Query<UnsubQuery>,
) -> Result<axum::response::Html<&'static str>, ApiError> {
    if let Some((org, email, _prospect)) = outreach::recipient_for_message(&state.pool, q.m).await?
    {
        outreach::suppress(&state.pool, org, &email, "unsubscribe").await?;
    }
    Ok(axum::response::Html(UNSUB_HTML))
}

const UNSUB_HTML: &str = "<!doctype html><meta charset=\"utf-8\"><title>Unsubscribed</title>\
<div style=\"font:16px/1.6 system-ui,sans-serif;max-width:32rem;margin:4rem auto;padding:0 1rem\">\
<h1>You're unsubscribed</h1><p>You won't receive any more outreach emails about this. \
It can take a moment to take effect. Sent in error? You can ignore this page.</p></div>";

// ─── Outreach inbound: a prospect replied (S4) ────────────────────────────────
#[derive(Deserialize)]
pub struct InboundQuery {
    pub s: Option<String>,
}

/// Constant-time byte compare (webhook secret).
fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Find the `reply+<prospect_uuid>@…` recipient token anywhere in the inbound
/// payload (robust to the provider's exact field layout).
fn extract_prospect_id(payload: &serde_json::Value) -> Option<Uuid> {
    let s = payload.to_string();
    let start = s.find("reply+")? + "reply+".len();
    let rest = &s[start..];
    let end = rest.find('@')?;
    Uuid::parse_str(rest[..end].trim()).ok()
}

/// Best-effort text extraction across common inbound schemas.
fn extract_str(payload: &serde_json::Value, paths: &[&[&str]]) -> Option<String> {
    'paths: for path in paths {
        let mut cur = payload;
        for key in *path {
            match cur.get(key) {
                Some(v) => cur = v,
                None => continue 'paths,
            }
        }
        if let Some(t) = cur.as_str().filter(|t| !t.is_empty()) {
            return Some(t.chars().take(5000).collect());
        }
    }
    None
}

/// Inbound webhook for prospect replies (S4). Public, gated by a shared secret
/// in the `?s=` query (disabled → 404 when `OUTREACH_INBOUND_SECRET` is unset).
/// Matches the reply token → stops the sequence and forwards the reply to the
/// campaign owner. Always acks 200 on a matched/unmatched payload to avoid
/// provider retries; only auth failures are rejected.
pub async fn outreach_inbound(
    State(state): State<AppState>,
    Query(q): Query<InboundQuery>,
    Json(payload): Json<serde_json::Value>,
) -> StatusCode {
    let Some(secret) = std::env::var("OUTREACH_INBOUND_SECRET")
        .ok()
        .filter(|s| !s.is_empty())
    else {
        return StatusCode::NOT_FOUND;
    };
    if !ct_eq(q.s.unwrap_or_default().as_bytes(), secret.as_bytes()) {
        return StatusCode::UNAUTHORIZED;
    }
    let Some(prospect_id) = extract_prospect_id(&payload) else {
        tracing::warn!("outreach inbound: no reply token in payload");
        return StatusCode::OK;
    };
    let _ = outreach::advance_on_reply(&state.pool, prospect_id).await;
    // Forward the reply to the owner so they can answer directly. No target or no
    // owner → nothing to forward, still ack 200.
    let Ok(Some(t)) = outreach::reply_target(&state.pool, prospect_id).await else {
        return StatusCode::OK;
    };
    let Some(owner) = t.owner_email else {
        return StatusCode::OK;
    };
    let reply_body = extract_str(
        &payload,
        &[
            &["text"][..],
            &["data", "text"][..],
            &["plain"][..],
            &["html"][..],
            &["data", "html"][..],
        ],
    )
    .unwrap_or_default();
    let from_addr =
        extract_str(&payload, &[&["from"][..], &["data", "from"][..]]).unwrap_or_default();
    let subject = format!("Reply from {} to your outreach", t.prospect_domain);
    let text = format!(
        "{reply_body}\n\n— Reply from {} ({from_addr}), forwarded by LinkPulz. \
Reply to this email to answer them directly.",
        t.prospect_domain
    );
    let reply_to = t.contact_email.unwrap_or(from_addr);
    let _ = notifications::resend::forward_reply(&owner, &subject, &text, &reply_to).await;
    StatusCode::OK
}

// ─── Outreach sequence: status (for the drawer) ───────────────────────────────
#[derive(Serialize)]
pub struct OutreachStepView {
    pub step: i16,
    pub status: String,
    pub scheduled_at: String,
    pub sent_at: Option<String>,
}

#[derive(Serialize)]
pub struct OutreachStatusView {
    /// Whether the server has outreach sending configured (gates the UI button).
    pub configured: bool,
    pub messages: Vec<OutreachStepView>,
}

/// Read a prospect's outreach sequence status (steps + whether sending is
/// configured). Used by the drawer to show "Start sequence" vs. progress.
pub async fn prospect_outreach_status(
    State(state): State<AppState>,
    Path((org_id, prospect_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
) -> Result<Json<OutreachStatusView>, ApiError> {
    require_beta(&state)?;
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    let rows =
        outreach::list_for_prospect(&state.pool, scope.organization_id(), prospect_id).await?;
    Ok(Json(OutreachStatusView {
        configured: notifications::resend::outreach_configured(),
        messages: rows
            .into_iter()
            .map(|r| OutreachStepView {
                step: r.step,
                status: r.status,
                scheduled_at: r.scheduled_at.to_rfc3339(),
                sent_at: r.sent_at.map(|d| d.to_rfc3339()),
            })
            .collect(),
    }))
}

// ─── Convert a won prospect to a protected backlink ───────────────────────────
#[derive(Deserialize)]
pub struct ConvertBody {
    pub source_url: String,
    pub target_url: Option<String>,
    pub publisher_id: Option<Uuid>,
    pub cost: Option<f64>,
    pub currency: Option<String>,
    pub anchor: Option<String>,
    pub rel: Option<String>,
}

#[derive(Serialize)]
pub struct ConvertResult {
    pub prospect: ProspectView,
    pub backlink_id: Uuid,
}

pub async fn convert_prospect(
    State(state): State<AppState>,
    Path((org_id, prospect_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<ConvertBody>,
) -> Result<(StatusCode, Json<ConvertResult>), ApiError> {
    require_beta(&state)?;
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    let cur = prospects::get(&state.pool, &scope, prospect_id).await?;
    let campaign = campaigns::get(&state.pool, &scope, cur.campaign_id).await?;

    let source_url = body.source_url.trim();
    if !is_http(source_url) {
        return Err(ApiError::Validation(
            "source_url must start with http:// or https://".into(),
        ));
    }
    // Target: explicit, else the prospect's, else the campaign's headline page.
    let target_owned = body
        .target_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| cur.target_url.clone())
        .or_else(|| campaign.primary_target_url.clone())
        .ok_or_else(|| {
            ApiError::Validation("target_url is required (none on the prospect or campaign)".into())
        })?;
    if !is_http(&target_owned) {
        return Err(ApiError::Validation(
            "target_url must start with http:// or https://".into(),
        ));
    }
    let rel = body.rel.as_deref().unwrap_or("dofollow");
    if !REL_TYPES.contains(&rel) {
        return Err(ApiError::Validation(format!("unknown rel {rel:?}")));
    }
    // Cost: explicit, else the prospect's estimate. Currency must accompany a cost.
    let (cost, currency) = match (body.cost, body.currency.as_deref()) {
        (Some(_), None) => {
            return Err(ApiError::Validation(
                "currency is required when cost is set".into(),
            ));
        }
        (Some(c), Some(cur_code)) => (Some(money(c)?), Some(norm_currency(cur_code)?)),
        (None, _) => (cur.estimated_cost.clone(), cur.currency.clone()),
    };
    let anchor = body
        .anchor
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    // A link we deliberately built through a campaign is protected investment.
    let reason = if cost.is_some() {
        "economic_value"
    } else {
        "strategic"
    };

    let bl = backlinks::create(
        &state.pool,
        &scope,
        Uuid::now_v7(),
        backlinks::NewBacklink {
            project_id: campaign.project_id,
            campaign_id: Some(cur.campaign_id),
            publisher_id: body.publisher_id,
            source_url,
            target_url: &target_owned,
            expected_anchor_text: anchor,
            expected_rel_type: rel,
            cost: cost.as_deref(),
            currency: currency.as_deref(),
            warranty_type: "none",
            warranty_value: None,
            warranty_expiration: None,
            monitoring_enabled: true,
            monitoring_level: "protected",
            monitoring_reason: Some(reason),
            check_frequency: None,
        },
    )
    .await?;

    // Mark the prospect won and link it to the backlink it became.
    let (score, score_reason) = score_for(cur.competitor_count, true, cost.is_some(), "won");
    let p = prospects::update(
        &state.pool,
        &scope,
        prospect_id,
        prospects::ProspectUpdate {
            stage: "won",
            contact_name: cur.contact_name.as_deref(),
            contact_email: cur.contact_email.as_deref(),
            target_url: Some(target_owned.as_str()),
            estimated_cost: cost.as_deref(),
            currency: currency.as_deref(),
            notes: cur.notes.as_deref(),
            score,
            score_reason: &score_reason,
            last_contacted_at: cur.last_contacted_at,
            won_backlink_id: Some(bl.id),
        },
    )
    .await?;

    let _ = audit::log(
        &state.pool,
        Some(org_id),
        Some(user.id),
        Some(user.email.as_str()),
        "prospect_converted",
        Some("prospect"),
        Some(prospect_id),
        None,
        Some(json!({ "backlink_id": bl.id, "campaign_id": cur.campaign_id })),
    )
    .await;
    let _ = analytics::record(
        &state.pool,
        "prospect_converted",
        Some(user.id),
        Some(org_id),
        None,
        json!({ "prospect_id": prospect_id, "campaign_id": cur.campaign_id }),
    )
    .await;

    Ok((
        StatusCode::CREATED,
        Json(ConvertResult {
            prospect: p.into(),
            backlink_id: bl.id,
        }),
    ))
}

// ─── Campaign summary (rollup + budget/spend + pipeline counts) ───────────────
#[derive(Serialize)]
pub struct StageCount {
    pub stage: String,
    pub count: i64,
}

#[derive(Serialize)]
pub struct CampaignSummaryView {
    pub campaign_id: Uuid,
    pub total: i64,
    pub live: i64,
    pub lost: i64,
    pub needs_attention: i64,
    pub removal_rate: f64,
    pub target_backlinks: Option<i32>,
    pub target_progress: Option<f64>,
    pub budget: Option<String>,
    pub currency: Option<String>,
    pub spend: Vec<super::ops_routes::CurrencyAmount>,
    pub prospects_total: i64,
    pub stage_counts: Vec<StageCount>,
}

pub async fn campaign_summary(
    State(state): State<AppState>,
    Path((org_id, campaign_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
) -> Result<Json<CampaignSummaryView>, ApiError> {
    require_beta(&state)?;
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    let campaign = campaigns::get(&state.pool, &scope, campaign_id).await?;
    let counts = campaigns::rollup(&state.pool, &scope, campaign_id).await?;
    let spend = campaigns::spend(&state.pool, &scope, campaign_id).await?;
    let stage_rows = prospects::stage_counts(&state.pool, &scope, campaign_id).await?;

    let target_progress = campaign
        .target_backlinks
        .and_then(|t| (t > 0).then(|| (counts.live as f64 / t as f64).min(1.0)));
    let removal_rate = if counts.total > 0 {
        counts.lost as f64 / counts.total as f64
    } else {
        0.0
    };
    let prospects_total: i64 = stage_rows.iter().map(|(_, n)| n).sum();
    let stage_counts = stage_rows
        .into_iter()
        .map(|(stage, count)| StageCount { stage, count })
        .collect();

    Ok(Json(CampaignSummaryView {
        campaign_id,
        total: counts.total,
        live: counts.live,
        lost: counts.lost,
        needs_attention: counts.attention,
        removal_rate,
        target_backlinks: campaign.target_backlinks,
        target_progress,
        budget: campaign.budget,
        currency: campaign.currency,
        spend: spend
            .into_iter()
            .map(|(currency, amount)| super::ops_routes::CurrencyAmount { currency, amount })
            .collect(),
        prospects_total,
        stage_counts,
    }))
}
