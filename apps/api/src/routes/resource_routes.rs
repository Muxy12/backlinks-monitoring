//! Tenant-scoped resource routes: clients (§20) and projects (§21), nested
//! under `/v1/organizations/{org_id}/...` (docs/09 §1.1). Every handler resolves
//! `OrgScope` from the path org id via `authorize_org` — the `org_id` is an
//! assertion to be checked against membership, never a trusted input (§18). A
//! non-member gets 404 (ADR-0004); a member lacking the write role gets 403.

use crate::error::ApiError;
use crate::session::{check_csrf, current};
use crate::state::AppState;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum_extra::extract::cookie::CookieJar;
use common::Role;
use db::authorize_org;
use db::repositories::{clients, projects};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Clients ─────────────────────────────────────────────────────────────────
#[derive(Debug, Serialize)]
pub struct ClientView {
    pub id: Uuid,
    pub name: String,
    pub company_name: Option<String>,
    pub website: Option<String>,
}

impl From<db::entities::Client> for ClientView {
    fn from(c: db::entities::Client) -> Self {
        ClientView {
            id: c.id,
            name: c.name,
            company_name: c.company_name,
            website: c.website,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateClientBody {
    pub name: String,
}

pub async fn create_client(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<CreateClientBody>,
) -> Result<(StatusCode, Json<ClientView>), ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?; // members and up may create (docs/05 §2)

    let name = body.name.trim();
    if name.is_empty() || name.len() > 200 {
        return Err(ApiError::Validation("name must be 1–200 characters".into()));
    }
    let client = clients::create(&state.pool, &scope, Uuid::now_v7(), name).await?;
    Ok((StatusCode::CREATED, Json(client.into())))
}

pub async fn list_clients(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
) -> Result<Json<Vec<ClientView>>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    let rows = clients::list(&state.pool, &scope).await?;
    Ok(Json(rows.into_iter().map(ClientView::from).collect()))
}

// ─── Projects ────────────────────────────────────────────────────────────────
#[derive(Debug, Serialize)]
pub struct ProjectView {
    pub id: Uuid,
    pub name: String,
    pub domain: String,
    pub check_frequency_secs: i64,
    pub status: String,
}

impl From<db::entities::Project> for ProjectView {
    fn from(p: db::entities::Project) -> Self {
        ProjectView {
            id: p.id,
            name: p.name,
            domain: p.domain,
            check_frequency_secs: p.check_frequency_secs,
            status: p.status,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateProjectBody {
    pub name: String,
    pub domain: String,
    /// Optional; defaults to 7 days. Bounded to the schema's 1h–180d range.
    pub check_frequency_hours: Option<i64>,
}

pub async fn create_project(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<CreateProjectBody>,
) -> Result<(StatusCode, Json<ProjectView>), ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;

    let name = body.name.trim();
    let domain = body.domain.trim();
    if name.is_empty() || name.len() > 200 {
        return Err(ApiError::Validation("name must be 1–200 characters".into()));
    }
    if domain.is_empty() {
        return Err(ApiError::Validation("domain is required".into()));
    }
    let hours = body.check_frequency_hours.unwrap_or(24 * 7);
    if !(1..=24 * 180).contains(&hours) {
        return Err(ApiError::Validation(
            "check frequency must be 1 hour – 180 days".into(),
        ));
    }
    let project = projects::create(
        &state.pool,
        &scope,
        Uuid::now_v7(),
        name,
        domain,
        hours * 3600,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(project.into())))
}

pub async fn list_projects(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
) -> Result<Json<Vec<ProjectView>>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    let rows = projects::list(&state.pool, &scope).await?;
    Ok(Json(rows.into_iter().map(ProjectView::from).collect()))
}

pub async fn get_project(
    State(state): State<AppState>,
    Path((org_id, project_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
) -> Result<Json<ProjectView>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    let project = projects::get(&state.pool, &scope, project_id).await?;
    Ok(Json(project.into()))
}

// ─── Backlinks (§23) ─────────────────────────────────────────────────────────
#[derive(Debug, Serialize)]
pub struct BacklinkView {
    pub id: Uuid,
    pub source_url: String,
    pub target_url: String,
    pub source_domain: String,
    pub target_domain: String,
    pub expected_anchor_text: Option<String>,
    pub expected_rel_type: String,
    pub status: String,
    pub cost: Option<String>,
    pub currency: Option<String>,
}

impl From<db::entities::Backlink> for BacklinkView {
    fn from(b: db::entities::Backlink) -> Self {
        BacklinkView {
            id: b.id,
            source_url: b.source_url,
            target_url: b.target_url,
            source_domain: b.source_domain,
            target_domain: b.target_domain,
            expected_anchor_text: b.expected_anchor_text,
            expected_rel_type: b.expected_rel_type,
            status: b.status,
            cost: b.cost,
            currency: b.currency,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateBacklinkBody {
    pub project_id: Uuid,
    pub source_url: String,
    pub target_url: String,
    pub expected_anchor_text: Option<String>,
    pub expected_rel_type: Option<String>,
    pub cost: Option<f64>,
    pub currency: Option<String>,
}

fn is_http_url(s: &str) -> bool {
    let s = s.trim();
    s.starts_with("http://") || s.starts_with("https://")
}

pub async fn create_backlink(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<CreateBacklinkBody>,
) -> Result<(StatusCode, Json<BacklinkView>), ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;

    if !is_http_url(&body.source_url) || !is_http_url(&body.target_url) {
        return Err(ApiError::Validation(
            "source_url and target_url must be http(s) URLs".into(),
        ));
    }
    let rel = body.expected_rel_type.as_deref().unwrap_or("dofollow");
    if !matches!(rel, "dofollow" | "nofollow" | "sponsored" | "ugc") {
        return Err(ApiError::Validation(
            "expected_rel_type must be dofollow/nofollow/sponsored/ugc".into(),
        ));
    }
    let cost_str = match (body.cost, body.currency.as_deref()) {
        (Some(_), None) => {
            return Err(ApiError::Validation(
                "currency is required when cost is set".into(),
            ));
        }
        (Some(c), Some(_)) if c < 0.0 => {
            return Err(ApiError::Validation("cost cannot be negative".into()));
        }
        (Some(c), Some(_)) => Some(format!("{c:.2}")),
        (None, _) => None,
    };
    // The project must belong to this org (returns 404 otherwise).
    db::repositories::projects::get(&state.pool, &scope, body.project_id).await?;

    let anchor = body
        .expected_anchor_text
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let new = db::repositories::backlinks::NewBacklink {
        project_id: body.project_id,
        source_url: body.source_url.trim(),
        target_url: body.target_url.trim(),
        expected_anchor_text: anchor,
        expected_rel_type: rel,
        cost: cost_str.as_deref(),
        currency: body.currency.as_deref(),
    };
    let backlink = db::repositories::backlinks::create(&state.pool, &scope, Uuid::now_v7(), new)
        .await
        .map_err(|e| match e {
            db::DbError::Config(_) => ApiError::Validation("could not parse a URL".into()),
            other => other.into(),
        })?;
    Ok((StatusCode::CREATED, Json(backlink.into())))
}

#[derive(Debug, Deserialize)]
pub struct ListBacklinksQuery {
    pub project_id: Uuid,
    pub limit: Option<i64>,
}

pub async fn list_backlinks(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    axum::extract::Query(q): axum::extract::Query<ListBacklinksQuery>,
    jar: CookieJar,
) -> Result<Json<Vec<BacklinkView>>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    let rows = db::repositories::backlinks::list_for_project(
        &state.pool,
        &scope,
        q.project_id,
        q.limit.unwrap_or(50),
    )
    .await?;
    Ok(Json(rows.into_iter().map(BacklinkView::from).collect()))
}

pub async fn get_backlink(
    State(state): State<AppState>,
    Path((org_id, backlink_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
) -> Result<Json<BacklinkView>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    let backlink = db::repositories::backlinks::get(&state.pool, &scope, backlink_id).await?;
    Ok(Json(backlink.into()))
}

/// POST /v1/organizations/{org_id}/backlinks/{backlink_id}/check — enqueue a
/// priority check (§36). Never crawls in-request (§9, §57.5): returns 202 with
/// the job id. Idempotent — repeated presses collapse to one pending job.
pub async fn check_backlink_now(
    State(state): State<AppState>,
    Path((org_id, backlink_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;

    // Confirm the backlink is in scope (404 otherwise) before enqueuing.
    db::repositories::backlinks::get(&state.pool, &scope, backlink_id).await?;

    let key = jobs::check_now_key(backlink_id);
    let payload = serde_json::json!({ "backlink_id": backlink_id });
    let job_id = db::repositories::jobs::enqueue(
        &state.pool,
        Some(scope.organization_id()),
        jobs::JobKind::CheckBacklinkNow,
        payload,
        Some(&key),
    )
    .await?;

    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({ "queued": true, "job_id": job_id })),
    ))
}

// ─── Backlink events (§30) + dashboard (§34) ─────────────────────────────────
#[derive(Debug, Serialize)]
pub struct EventView {
    pub event_type: String,
    pub previous_data: Option<serde_json::Value>,
    pub current_data: Option<serde_json::Value>,
    pub detected_at: String,
}

pub async fn get_backlink_events(
    State(state): State<AppState>,
    Path((org_id, backlink_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
) -> Result<Json<Vec<EventView>>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    // Confirm the backlink is in scope first (404 otherwise).
    db::repositories::backlinks::get(&state.pool, &scope, backlink_id).await?;
    let rows =
        db::repositories::events::list_for_backlink(&state.pool, &scope, backlink_id, 200).await?;
    Ok(Json(
        rows.into_iter()
            .map(|e| EventView {
                event_type: e.event_type,
                previous_data: e.previous_data,
                current_data: e.current_data,
                detected_at: e.detected_at.to_rfc3339(),
            })
            .collect(),
    ))
}

#[derive(Debug, Serialize)]
pub struct MoneyByCurrency {
    pub currency: String,
    pub amount: String,
}

#[derive(Debug, Serialize)]
pub struct DashboardView {
    /// status -> count for every non-archived backlink.
    pub counts: std::collections::BTreeMap<String, i64>,
    pub monitored: i64,
    pub needs_attention: i64,
    pub value_at_risk: Vec<MoneyByCurrency>,
    pub tracked_spend: Vec<MoneyByCurrency>,
}

pub async fn dashboard(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
) -> Result<Json<DashboardView>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;

    let counts_rows = db::repositories::dashboard::status_counts(&state.pool, &scope).await?;
    let mut counts = std::collections::BTreeMap::new();
    let mut monitored = 0i64;
    let mut needs_attention = 0i64;
    const ATTENTION: &[&str] = &[
        "lost",
        "changed",
        "nofollow",
        "source_error",
        "target_error",
        "redirected",
    ];
    for (status, n) in counts_rows {
        monitored += n;
        if ATTENTION.contains(&status.as_str()) {
            needs_attention += n;
        }
        counts.insert(status, n);
    }

    let to_money = |rows: Vec<(String, String)>| {
        rows.into_iter()
            .map(|(currency, amount)| MoneyByCurrency { currency, amount })
            .collect::<Vec<_>>()
    };
    let value_at_risk =
        to_money(db::repositories::dashboard::value_at_risk(&state.pool, &scope).await?);
    let tracked_spend =
        to_money(db::repositories::dashboard::tracked_spend(&state.pool, &scope).await?);

    Ok(Json(DashboardView {
        counts,
        monitored,
        needs_attention,
        value_at_risk,
        tracked_spend,
    }))
}
