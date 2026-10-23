//! Spot Finder — SERP search & spots (Lot B).
//!
//! The kept keywords (Lot A) feed a SERP lookup: for each keyword we ask who
//! ranks (DataForSEO), aggregate the ranking domains, and store them as "spots"
//! — relevance-first prospects. The search runs as a background job
//! (`find_spots`); these routes start it, report status, and list the spots.
//!
//! Gated by the same `spot_finder` flag as the keyword routes (404 when off).
//! The SERP provider itself is separately env-gated: search returns a clear
//! error (and the UI disables the action via `me.features.dataforseo`) when it
//! is not configured.

use crate::error::ApiError;
use crate::routes::spot_keyword_routes::configured;
use crate::session::{check_csrf, current};
use crate::state::AppState;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum_extra::extract::cookie::CookieJar;
use common::Role;
use db::authorize_org;
use db::repositories::{projects, spots};
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
pub struct RunView {
    pub id: Uuid,
    pub status: String,
    pub locale: String,
    pub keywords_queried: i32,
    pub serp_queries: i32,
    pub spots_found: i32,
    pub error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<db::entities::SpotSearchRun> for RunView {
    fn from(r: db::entities::SpotSearchRun) -> Self {
        RunView {
            id: r.id,
            status: r.status,
            locale: r.locale,
            keywords_queried: r.keywords_queried,
            serp_queries: r.serp_queries,
            spots_found: r.spots_found,
            error: r.error,
            created_at: r.created_at.to_rfc3339(),
            updated_at: r.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Serialize)]
pub struct SpotView {
    pub id: Uuid,
    pub domain: String,
    pub keyword_count: i32,
    pub best_position: Option<i32>,
    pub avg_position: Option<f64>,
    pub example_url: Option<String>,
    pub example_keyword: Option<String>,
    // Enrichment (Lot C) — null until the spot is enriched.
    pub domain_rank: Option<i32>,
    pub referring_domains: Option<i32>,
    pub spam_score: Option<i32>,
    pub organic_keywords: Option<i32>,
    pub organic_traffic: Option<i64>,
    pub quality_score: Option<f64>,
    pub enriched: bool,
    // Marketplace match (Lot D) — null until matched.
    pub marketplace: Option<String>,
    pub buy_price_cents: Option<i64>,
    pub buy_currency: Option<String>,
    pub marketplace_url: Option<String>,
    pub buyable: bool,
}

impl From<db::entities::Spot> for SpotView {
    fn from(s: db::entities::Spot) -> Self {
        SpotView {
            id: s.id,
            domain: s.domain,
            keyword_count: s.keyword_count,
            best_position: s.best_position,
            avg_position: s.avg_position,
            example_url: s.example_url,
            example_keyword: s.example_keyword,
            domain_rank: s.domain_rank,
            referring_domains: s.referring_domains,
            spam_score: s.spam_score,
            organic_keywords: s.organic_keywords,
            organic_traffic: s.organic_traffic,
            quality_score: s.quality_score,
            enriched: s.enriched_at.is_some(),
            buyable: s.marketplace.is_some(),
            marketplace: s.marketplace,
            buy_price_cents: s.buy_price_cents,
            buy_currency: s.buy_currency,
            marketplace_url: s.marketplace_url,
        }
    }
}

#[derive(Serialize)]
pub struct EnrichRunView {
    pub id: Uuid,
    pub status: String,
    pub spots_enriched: i32,
    pub provider_queries: i32,
    pub error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<db::entities::SpotEnrichRun> for EnrichRunView {
    fn from(r: db::entities::SpotEnrichRun) -> Self {
        EnrichRunView {
            id: r.id,
            status: r.status,
            spots_enriched: r.spots_enriched,
            provider_queries: r.provider_queries,
            error: r.error,
            created_at: r.created_at.to_rfc3339(),
            updated_at: r.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Serialize)]
pub struct MatchRunView {
    pub id: Uuid,
    pub status: String,
    pub spots_checked: i32,
    pub matches_found: i32,
    pub error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<db::entities::SpotMarketplaceRun> for MatchRunView {
    fn from(r: db::entities::SpotMarketplaceRun) -> Self {
        MatchRunView {
            id: r.id,
            status: r.status,
            spots_checked: r.spots_checked,
            matches_found: r.matches_found,
            error: r.error,
            created_at: r.created_at.to_rfc3339(),
            updated_at: r.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Serialize)]
pub struct ForumView {
    pub id: Uuid,
    pub url: String,
    pub domain: String,
    pub forum_kind: String,
    pub example_keyword: Option<String>,
    pub best_position: Option<i32>,
}

impl From<db::entities::SpotForum> for ForumView {
    fn from(f: db::entities::SpotForum) -> Self {
        ForumView {
            id: f.id,
            url: f.url,
            domain: f.domain,
            forum_kind: f.forum_kind,
            example_keyword: f.example_keyword,
            best_position: f.best_position,
        }
    }
}

#[derive(Serialize)]
pub struct SpotsResponse {
    pub run: Option<RunView>,
    pub enrich_run: Option<EnrichRunView>,
    pub match_run: Option<MatchRunView>,
    pub spots: Vec<SpotView>,
    pub forums: Vec<ForumView>,
}

/// `GET …/projects/{project_id}/spots` — the spots list plus the latest run
/// status. Any member may read.
pub async fn list_spots(
    State(state): State<AppState>,
    Path((org_id, project_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
) -> Result<Json<SpotsResponse>, ApiError> {
    gate(&state)?;
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Viewer)?;
    projects::get(&state.pool, &scope, project_id).await?;

    let run = spots::latest_run(&state.pool, &scope, project_id).await?;
    let enrich_run = spots::latest_enrich(&state.pool, &scope, project_id).await?;
    let match_run = spots::latest_match(&state.pool, &scope, project_id).await?;
    let rows = spots::list_spots(&state.pool, &scope, project_id).await?;
    let forums = spots::list_forums(&state.pool, &scope, project_id).await?;
    Ok(Json(SpotsResponse {
        run: run.map(RunView::from),
        enrich_run: enrich_run.map(EnrichRunView::from),
        match_run: match_run.map(MatchRunView::from),
        spots: rows.into_iter().map(SpotView::from).collect(),
        forums: forums.into_iter().map(ForumView::from).collect(),
    }))
}

#[derive(Deserialize)]
pub struct SearchBody {
    /// Locale for the SERP lookup: fr | en. Defaults to fr.
    pub locale: Option<String>,
}

/// `POST …/projects/{project_id}/spots/search` — run the SERP lookup over the
/// kept keywords. Member role. Paid (DataForSEO) — refused with a clear message
/// when the provider is not configured or there are no kept keywords.
pub async fn search_spots(
    State(state): State<AppState>,
    Path((org_id, project_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<SearchBody>,
) -> Result<(StatusCode, Json<RunView>), ApiError> {
    gate(&state)?;
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    projects::get(&state.pool, &scope, project_id).await?;

    if crawler::serp::SerpProvider::from_env().is_none() {
        return Err(ApiError::Validation(
            "DataForSEO is not configured — set DATAFORSEO_LOGIN and DATAFORSEO_PASSWORD".into(),
        ));
    }

    let locale = match body.locale.as_deref() {
        Some("en") => "en",
        _ => "fr",
    };

    // Single run at a time per project: return the in-flight one if present.
    if spots::search_in_flight(&state.pool, &scope, project_id).await?
        && let Some(existing) = spots::latest_run(&state.pool, &scope, project_id).await?
    {
        return Ok((StatusCode::ACCEPTED, Json(RunView::from(existing))));
    }

    // Require at least one kept keyword before spending anything.
    let kept = spots::kept_keywords(&state.pool, scope.organization_id(), project_id, 1).await?;
    if kept.is_empty() {
        return Err(ApiError::Validation(
            "keep at least one keyword before searching for spots".into(),
        ));
    }

    let run = spots::start_run(&state.pool, &scope, Uuid::now_v7(), project_id, locale).await?;
    let key = format!("find_spots:{}", run.id);
    db::repositories::jobs::enqueue(
        &state.pool,
        Some(scope.organization_id()),
        jobs::JobKind::FindSpots,
        serde_json::json!({ "run_id": run.id }),
        Some(&key),
    )
    .await?;

    Ok((StatusCode::ACCEPTED, Json(RunView::from(run))))
}

/// `POST …/projects/{project_id}/spots/enrich` — enrich the shortlist of spots
/// with authority/traffic metrics and compute the quality score (stage 2).
/// Member role. Paid (DataForSEO) — refused when the provider is unconfigured or
/// there are no spots to enrich yet.
pub async fn enrich_spots(
    State(state): State<AppState>,
    Path((org_id, project_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
) -> Result<(StatusCode, Json<EnrichRunView>), ApiError> {
    gate(&state)?;
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    projects::get(&state.pool, &scope, project_id).await?;

    if crawler::metrics::MetricsProvider::from_env().is_none() {
        return Err(ApiError::Validation(
            "DataForSEO is not configured — set DATAFORSEO_LOGIN and DATAFORSEO_PASSWORD".into(),
        ));
    }

    if spots::enrich_in_flight(&state.pool, &scope, project_id).await?
        && let Some(existing) = spots::latest_enrich(&state.pool, &scope, project_id).await?
    {
        return Ok((StatusCode::ACCEPTED, Json(EnrichRunView::from(existing))));
    }

    // Need spots before enriching them.
    let some = spots::spots_to_enrich(&state.pool, scope.organization_id(), project_id, 1).await?;
    if some.is_empty() {
        return Err(ApiError::Validation(
            "search for spots before enriching them".into(),
        ));
    }

    let run = spots::start_enrich(&state.pool, &scope, Uuid::now_v7(), project_id).await?;
    let key = format!("enrich_spots:{}", run.id);
    db::repositories::jobs::enqueue(
        &state.pool,
        Some(scope.organization_id()),
        jobs::JobKind::EnrichSpots,
        serde_json::json!({ "run_id": run.id }),
        Some(&key),
    )
    .await?;

    Ok((StatusCode::ACCEPTED, Json(EnrichRunView::from(run))))
}

/// `POST …/projects/{project_id}/spots/match` — cross-reference the spots with
/// the configured link-buying marketplaces (LinkAvista …) and mark the buyable
/// ones. Member role. Refused when no marketplace is configured or there are no
/// spots yet. Catalogue reads don't spend credit.
pub async fn match_spots(
    State(state): State<AppState>,
    Path((org_id, project_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
) -> Result<(StatusCode, Json<MatchRunView>), ApiError> {
    gate(&state)?;
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    projects::get(&state.pool, &scope, project_id).await?;

    if crate::routes::linkavista_routes::configured(&state.cfg) {
        // ok: at least one marketplace (LinkAvista) is configured
    } else {
        return Err(ApiError::Validation(
            "no link marketplace is configured (set LINKAVISTA_API_KEY)".into(),
        ));
    }

    if spots::match_in_flight(&state.pool, &scope, project_id).await?
        && let Some(existing) = spots::latest_match(&state.pool, &scope, project_id).await?
    {
        return Ok((StatusCode::ACCEPTED, Json(MatchRunView::from(existing))));
    }

    let some = spots::spots_to_enrich(&state.pool, scope.organization_id(), project_id, 1).await?;
    if some.is_empty() {
        return Err(ApiError::Validation(
            "search for spots before matching marketplaces".into(),
        ));
    }

    let run = spots::start_match(&state.pool, &scope, Uuid::now_v7(), project_id).await?;
    let key = format!("match_marketplaces:{}", run.id);
    db::repositories::jobs::enqueue(
        &state.pool,
        Some(scope.organization_id()),
        jobs::JobKind::MatchMarketplaces,
        serde_json::json!({ "run_id": run.id }),
        Some(&key),
    )
    .await?;

    Ok((StatusCode::ACCEPTED, Json(MatchRunView::from(run))))
}
