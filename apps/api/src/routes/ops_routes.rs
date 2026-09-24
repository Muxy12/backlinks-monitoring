//! Campaigns (§22) and publishers (§31): the standard create/list/get, plus the
//! read-time rollups that make them useful — status counts, spend by currency,
//! and (for publishers) average lifespan and removal rate. Every handler scopes
//! to the path org via `authorize_org`, so a cross-tenant id yields 404
//! (ADR-0004). Rollups are computed on read (docs/03 §3.3), never denormalized.

use crate::error::ApiError;
use crate::session::{check_csrf, current};
use crate::state::AppState;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum_extra::extract::cookie::CookieJar;
use common::Role;
use db::authorize_org;
use db::repositories::{campaigns, publishers};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A money amount tagged with its currency. Amounts are never summed across
/// currencies (§33/§34) — each currency is its own line.
#[derive(Debug, Serialize)]
pub struct CurrencyAmount {
    pub currency: String,
    pub amount: String,
}

fn to_amounts(rows: Vec<(String, String)>) -> Vec<CurrencyAmount> {
    rows.into_iter()
        .map(|(currency, amount)| CurrencyAmount { currency, amount })
        .collect()
}

/// `lost / total`, in `0.0..=1.0`; 0 when nothing is tracked yet.
fn removal_rate(lost: i64, total: i64) -> f64 {
    if total <= 0 {
        0.0
    } else {
        lost as f64 / total as f64
    }
}

// ─── Campaigns (§22) ──────────────────────────────────────────────────────────
#[derive(Debug, Serialize)]
pub struct CampaignView {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub target_backlinks: Option<i32>,
    pub budget: Option<String>,
    pub currency: Option<String>,
}

impl From<db::entities::Campaign> for CampaignView {
    fn from(c: db::entities::Campaign) -> Self {
        CampaignView {
            id: c.id,
            project_id: c.project_id,
            name: c.name,
            description: c.description,
            status: c.status,
            target_backlinks: c.target_backlinks,
            budget: c.budget,
            currency: c.currency,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateCampaignBody {
    pub project_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub target_backlinks: Option<i32>,
    /// Budget requires a currency; a bare number would be undisplayable (§33).
    pub budget: Option<f64>,
    pub currency: Option<String>,
}

pub async fn create_campaign(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<CreateCampaignBody>,
) -> Result<(StatusCode, Json<CampaignView>), ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;

    let name = body.name.trim();
    if name.is_empty() || name.len() > 200 {
        return Err(ApiError::Validation("name must be 1–200 characters".into()));
    }
    let budget = match (body.budget, body.currency.as_deref()) {
        (Some(_), None) => {
            return Err(ApiError::Validation(
                "currency is required when budget is set".into(),
            ));
        }
        (Some(b), Some(_)) if b < 0.0 => {
            return Err(ApiError::Validation("budget cannot be negative".into()));
        }
        (Some(b), Some(_)) => Some(format!("{b:.2}")),
        (None, _) => None,
    };
    if let Some(t) = body.target_backlinks
        && t <= 0
    {
        return Err(ApiError::Validation(
            "target_backlinks must be positive".into(),
        ));
    }
    // The campaign's project must belong to this org (404 otherwise).
    db::repositories::projects::get(&state.pool, &scope, body.project_id).await?;

    let description = body
        .description
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let new = campaigns::NewCampaign {
        project_id: body.project_id,
        name,
        description,
        target_backlinks: body.target_backlinks,
        budget: budget.as_deref(),
        currency: body.currency.as_deref(),
    };
    let campaign = campaigns::create(&state.pool, &scope, Uuid::now_v7(), new).await?;
    Ok((StatusCode::CREATED, Json(campaign.into())))
}

pub async fn list_campaigns(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
) -> Result<Json<Vec<CampaignView>>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    let rows = campaigns::list(&state.pool, &scope).await?;
    Ok(Json(rows.into_iter().map(CampaignView::from).collect()))
}

pub async fn get_campaign(
    State(state): State<AppState>,
    Path((org_id, campaign_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
) -> Result<Json<CampaignView>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    let campaign = campaigns::get(&state.pool, &scope, campaign_id).await?;
    Ok(Json(campaign.into()))
}

pub async fn delete_campaign(
    State(state): State<AppState>,
    Path((org_id, campaign_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    campaigns::delete(&state.pool, &scope, campaign_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize)]
pub struct CampaignRollupView {
    pub campaign_id: Uuid,
    pub total: i64,
    pub live: i64,
    pub lost: i64,
    pub needs_attention: i64,
    pub removal_rate: f64,
    pub avg_lifespan_days: Option<f64>,
    pub target_backlinks: Option<i32>,
    /// Live links toward target, in `0.0..=1.0`; `None` when no target is set.
    pub target_progress: Option<f64>,
    pub spend: Vec<CurrencyAmount>,
}

pub async fn campaign_rollup(
    State(state): State<AppState>,
    Path((org_id, campaign_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
) -> Result<Json<CampaignRollupView>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    // 404s if the campaign is not in this org — before any aggregate runs.
    let campaign = campaigns::get(&state.pool, &scope, campaign_id).await?;
    let counts = campaigns::rollup(&state.pool, &scope, campaign_id).await?;
    let spend = campaigns::spend(&state.pool, &scope, campaign_id).await?;

    let target_progress = campaign
        .target_backlinks
        .and_then(|t| (t > 0).then(|| (counts.live as f64 / t as f64).min(1.0)));
    Ok(Json(CampaignRollupView {
        campaign_id,
        total: counts.total,
        live: counts.live,
        lost: counts.lost,
        needs_attention: counts.attention,
        removal_rate: removal_rate(counts.lost, counts.total),
        avg_lifespan_days: counts.avg_lifespan_days,
        target_backlinks: campaign.target_backlinks,
        target_progress,
        spend: to_amounts(spend),
    }))
}

// ─── Publishers (§31) ─────────────────────────────────────────────────────────
#[derive(Debug, Serialize)]
pub struct PublisherView {
    pub id: Uuid,
    pub name: String,
    pub domain: String,
    pub contact_name: Option<String>,
    pub contact_email: Option<String>,
}

impl From<db::entities::Publisher> for PublisherView {
    fn from(p: db::entities::Publisher) -> Self {
        PublisherView {
            id: p.id,
            name: p.name,
            domain: p.domain,
            contact_name: p.contact_name,
            contact_email: p.contact_email,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreatePublisherBody {
    pub name: String,
    pub domain: String,
    pub contact_name: Option<String>,
    pub contact_email: Option<String>,
}

pub async fn create_publisher(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<CreatePublisherBody>,
) -> Result<(StatusCode, Json<PublisherView>), ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;

    let name = body.name.trim();
    let domain = body.domain.trim();
    if name.is_empty() || name.len() > 200 {
        return Err(ApiError::Validation("name must be 1–200 characters".into()));
    }
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
    let new = publishers::NewPublisher {
        name,
        domain,
        contact_name,
        contact_email,
    };
    let publisher = publishers::create(&state.pool, &scope, Uuid::now_v7(), new).await?;
    Ok((StatusCode::CREATED, Json(publisher.into())))
}

pub async fn list_publishers(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
) -> Result<Json<Vec<PublisherView>>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    let rows = publishers::list(&state.pool, &scope).await?;
    Ok(Json(rows.into_iter().map(PublisherView::from).collect()))
}

pub async fn get_publisher(
    State(state): State<AppState>,
    Path((org_id, publisher_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
) -> Result<Json<PublisherView>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    let publisher = publishers::get(&state.pool, &scope, publisher_id).await?;
    Ok(Json(publisher.into()))
}

pub async fn delete_publisher(
    State(state): State<AppState>,
    Path((org_id, publisher_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    publishers::delete(&state.pool, &scope, publisher_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize)]
pub struct PublisherRollupView {
    pub publisher_id: Uuid,
    pub total: i64,
    pub live: i64,
    pub lost: i64,
    pub needs_attention: i64,
    /// `lost / total` — the §31 removal rate, in `0.0..=1.0`.
    pub removal_rate: f64,
    /// Average time a now-lost link stayed live, in days; `None` if none lost.
    pub avg_lifespan_days: Option<f64>,
    pub spend: Vec<CurrencyAmount>,
}

pub async fn publisher_rollup(
    State(state): State<AppState>,
    Path((org_id, publisher_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
) -> Result<Json<PublisherRollupView>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    publishers::get(&state.pool, &scope, publisher_id).await?; // 404 if not ours
    let counts = publishers::rollup(&state.pool, &scope, publisher_id).await?;
    let spend = publishers::spend(&state.pool, &scope, publisher_id).await?;
    Ok(Json(PublisherRollupView {
        publisher_id,
        total: counts.total,
        live: counts.live,
        lost: counts.lost,
        needs_attention: counts.attention,
        removal_rate: removal_rate(counts.lost, counts.total),
        avg_lifespan_days: counts.avg_lifespan_days,
        spend: to_amounts(spend),
    }))
}
