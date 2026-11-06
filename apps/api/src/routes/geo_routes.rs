//! GEO pillar — "cited by AI" (Phase 2).
//!
//! Manage the queries we track for a project, trigger a capture of the AI
//! answers (DataForSEO AI Overviews, as background `capture_geo` jobs), and read
//! back the cited domains with their explainable GEO score and whether each is
//! placeable. Gated by the same `spot_finder` flag as the rest of the
//! acquisition suite (404 when off); the provider is separately env-gated.
//!
//! A refresh spends provider units, so it is quota-gated at enqueue: over the
//! plan's GEO allowance → 429, and it never deletes data or touches monitoring.

use crate::error::ApiError;
use crate::routes::spot_keyword_routes::configured;
use crate::session::{check_csrf, current};
use crate::state::AppState;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum_extra::extract::cookie::CookieJar;
use backlio_core::{GeoSignals, SpotSignals, geo_score};
use common::Role;
use db::authorize_org;
use db::repositories::{geo, projects, usage};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

fn gate(state: &AppState) -> Result<(), ApiError> {
    if configured(&state.cfg) {
        Ok(())
    } else {
        Err(ApiError::NotFound)
    }
}

#[derive(Serialize)]
pub struct GeoQueryView {
    pub id: Uuid,
    pub query: String,
    pub locale: String,
    pub engine: String,
    pub enabled: bool,
}

impl From<db::entities::GeoQuery> for GeoQueryView {
    fn from(q: db::entities::GeoQuery) -> Self {
        GeoQueryView {
            id: q.id,
            query: q.query,
            locale: q.locale,
            engine: q.engine,
            enabled: q.enabled,
        }
    }
}

#[derive(Serialize)]
pub struct GeoQueriesResponse {
    pub queries: Vec<GeoQueryView>,
}

/// `GET …/projects/{project_id}/geo/queries` — the tracked GEO queries.
pub async fn list_queries(
    State(state): State<AppState>,
    Path((org_id, project_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
) -> Result<Json<GeoQueriesResponse>, ApiError> {
    gate(&state)?;
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Viewer)?;
    projects::get(&state.pool, &scope, project_id).await?;

    let queries = geo::list_queries(&state.pool, &scope, project_id).await?;
    Ok(Json(GeoQueriesResponse {
        queries: queries.into_iter().map(GeoQueryView::from).collect(),
    }))
}

#[derive(Deserialize)]
pub struct AddQueryBody {
    pub query: String,
    pub locale: Option<String>,
    pub engine: Option<String>,
}

/// `POST …/projects/{project_id}/geo/queries` — track a new GEO query.
pub async fn add_query(
    State(state): State<AppState>,
    Path((org_id, project_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<AddQueryBody>,
) -> Result<(StatusCode, Json<GeoQueryView>), ApiError> {
    gate(&state)?;
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    projects::get(&state.pool, &scope, project_id).await?;

    let query = body.query.trim();
    if query.is_empty() {
        return Err(ApiError::Validation("query must not be empty".into()));
    }
    // Only AI Overviews today; other engines slot in behind GeoProvider later.
    let engine = match body.engine.as_deref() {
        Some("ai_overview") | None => "ai_overview",
        Some(_) => return Err(ApiError::Validation("unsupported GEO engine".into())),
    };
    let locale = body.locale.as_deref().unwrap_or("en");

    let q = geo::add_query(&state.pool, &scope, project_id, query, locale, engine).await?;
    Ok((StatusCode::CREATED, Json(GeoQueryView::from(q))))
}

/// `DELETE …/projects/{project_id}/geo/queries/{query_id}` — stop tracking one.
pub async fn delete_query(
    State(state): State<AppState>,
    Path((org_id, _project_id, query_id)): Path<(Uuid, Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    gate(&state)?;
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;

    geo::delete_query(&state.pool, &scope, query_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize)]
pub struct GeoRefreshView {
    pub enqueued: i32,
    pub remaining: Option<i64>,
}

/// `POST …/projects/{project_id}/geo/refresh` — capture the AI answers for every
/// enabled query (background jobs). Quota-gated at enqueue.
pub async fn refresh(
    State(state): State<AppState>,
    Path((org_id, project_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
) -> Result<(StatusCode, Json<GeoRefreshView>), ApiError> {
    gate(&state)?;
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    projects::get(&state.pool, &scope, project_id).await?;

    if crawler::geo::GeoProvider::from_env().is_none() {
        return Err(ApiError::Validation(
            "DataForSEO is not configured — set DATAFORSEO_LOGIN and DATAFORSEO_PASSWORD".into(),
        ));
    }

    let queries = geo::enabled_queries(&state.pool, &scope, project_id).await?;
    if queries.is_empty() {
        return Err(ApiError::Validation(
            "add at least one GEO query before refreshing".into(),
        ));
    }

    // Each capture spends provider units → check the GEO quota before enqueue.
    // Block the whole refresh if it would exceed the plan's allowance (429);
    // never deletes data, never touches monitoring (§6).
    let spec = db::repositories::billing::plan_for_org(&state.pool, scope.organization_id())
        .await?
        .spec();
    let used = usage::geo_queries_this_period(&state.pool, scope.organization_id()).await? as u32;
    let want = queries.len() as u32;
    let decision = backlio_core::quota::check(spec.geo_query_limit, used, want);
    if !decision.allowed {
        return Err(ApiError::QuotaExceeded(format!(
            "GEO quota reached for this period ({} of {} used) — refreshing {} queries would exceed it. It resets next month, or upgrade your plan for more.",
            decision.used,
            decision.limit.unwrap_or(0),
            want,
        )));
    }

    // Idempotency key folds a day bucket (doc 23): a same-day re-refresh of a
    // query is a no-op, so we only bill and count the captures actually enqueued.
    let day = chrono::Utc::now().date_naive();
    let mut enqueued = 0i32;
    for q in &queries {
        let key = format!("capture_geo:{}:{}", q.id, day);
        let job = db::repositories::jobs::enqueue(
            &state.pool,
            Some(scope.organization_id()),
            jobs::JobKind::CaptureGeo,
            serde_json::json!({ "geo_query_id": q.id }),
            Some(&key),
        )
        .await?;
        if job.is_some() {
            enqueued += 1;
        }
    }
    if enqueued > 0 {
        usage::add_geo_queries(&state.pool, scope.organization_id(), enqueued).await?;
    }

    Ok((
        StatusCode::ACCEPTED,
        Json(GeoRefreshView {
            enqueued,
            remaining: decision.remaining.map(|r| r as i64),
        }),
    ))
}

#[derive(Serialize)]
pub struct GeoCitationView {
    pub domain: String,
    pub citation_count: i64,
    pub best_rank: Option<i32>,
    pub engine_count: i64,
    pub placeable: bool,
    /// The explainable GEO opportunity score (0–100) and its breakdown.
    pub score: f64,
    pub coverage: f64,
    pub prominence: f64,
    pub reach: f64,
    pub quality: f64,
}

#[derive(Serialize)]
pub struct GeoCitationsResponse {
    pub query_count: i64,
    pub domains: Vec<GeoCitationView>,
}

/// `GET …/projects/{project_id}/geo/citations` — the cited domains, each with
/// its GEO score (computed on the fly) and whether it is placeable.
pub async fn list_citations(
    State(state): State<AppState>,
    Path((org_id, project_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
) -> Result<Json<GeoCitationsResponse>, ApiError> {
    gate(&state)?;
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Viewer)?;
    projects::get(&state.pool, &scope, project_id).await?;

    let query_count = geo::count_queries(&state.pool, &scope, project_id).await?;
    let rows = geo::domain_citations(&state.pool, &scope, project_id).await?;

    let domains: Vec<GeoCitationView> = rows
        .into_iter()
        .map(|r| {
            let signals = GeoSignals {
                citation_count: r.citation_count.max(0) as u32,
                query_count: query_count.max(0) as u32,
                best_rank: r.best_rank.map(|v| v.max(1) as u32),
                engine_count: r.engine_count.max(0) as u32,
                placeable: r.placeable,
                domain: SpotSignals {
                    keyword_count: r.keyword_count.unwrap_or(0).max(0) as u32,
                    domain_rank: r.domain_rank,
                    referring_domains: None,
                    spam_score: r.spam_score,
                    organic_keywords: r.organic_keywords,
                    organic_traffic: r.organic_traffic,
                },
            };
            let s = geo_score(&signals);
            GeoCitationView {
                domain: r.cited_domain,
                citation_count: r.citation_count,
                best_rank: r.best_rank,
                engine_count: r.engine_count,
                placeable: r.placeable,
                score: s.total,
                coverage: s.coverage,
                prominence: s.prominence,
                reach: s.reach,
                quality: s.quality,
            }
        })
        .collect();

    Ok(Json(GeoCitationsResponse {
        query_count,
        domains,
    }))
}
