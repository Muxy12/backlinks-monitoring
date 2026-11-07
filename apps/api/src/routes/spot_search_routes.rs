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

/// One marketplace offer for a spot (price-compare). A spot may carry several.
#[derive(Serialize)]
pub struct OfferView {
    pub source: String,
    pub price_cents: Option<i64>,
    pub currency: Option<String>,
    pub dofollow: Option<bool>,
    pub content_included: Option<bool>,
    pub indexation_guarantee: Option<bool>,
    pub retention_months: Option<i32>,
    pub url: Option<String>,
}

impl From<db::entities::MarketplaceOffer> for OfferView {
    fn from(o: db::entities::MarketplaceOffer) -> Self {
        OfferView {
            source: o.source_slug,
            price_cents: o.price_cents,
            currency: o.currency,
            dofollow: o.dofollow,
            content_included: o.content_included,
            indexation_guarantee: o.indexation_guarantee,
            retention_months: o.retention_months,
            url: o.offer_url,
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
    /// All marketplace offers for this spot (cheapest first), for price-compare.
    /// The embedded `marketplace`/`buy_*` fields above mirror the cheapest one.
    pub offers: Vec<OfferView>,
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
            offers: Vec::new(),
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

    // Attach each spot's marketplace offers (price-compare), grouped by spot.
    let offers = spots::offers_for_project(&state.pool, &scope, project_id).await?;
    let mut by_spot: std::collections::HashMap<Uuid, Vec<OfferView>> =
        std::collections::HashMap::new();
    for o in offers {
        by_spot
            .entry(o.spot_id)
            .or_default()
            .push(OfferView::from(o));
    }
    let spots: Vec<SpotView> = rows
        .into_iter()
        .map(|s| {
            let mut v = SpotView::from(s);
            v.offers = by_spot.remove(&v.id).unwrap_or_default();
            v
        })
        .collect();

    Ok(Json(SpotsResponse {
        run: run.map(RunView::from),
        enrich_run: enrich_run.map(EnrichRunView::from),
        match_run: match_run.map(MatchRunView::from),
        spots,
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

    // Acquisition quota (§6): a discovery spends provider units, so it is
    // checked here, at enqueue. Over quota blocks THIS run (429) — it never
    // deletes data and never touches monitoring. Resets each calendar month.
    quota_gate(&state, scope.organization_id()).await?;

    let run = spots::start_run(&state.pool, &scope, Uuid::now_v7(), project_id, locale).await?;
    let key = format!("find_spots:{}", run.id);
    let enqueued = db::repositories::jobs::enqueue(
        &state.pool,
        Some(scope.organization_id()),
        jobs::JobKind::FindSpots,
        serde_json::json!({ "run_id": run.id }),
        Some(&key),
    )
    .await?;
    // Count the run only if a job was actually enqueued (not a dedupe no-op).
    if enqueued.is_some() {
        db::repositories::usage::add_discoveries(&state.pool, scope.organization_id(), 1).await?;
    }

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
    let some =
        spots::spots_to_enrich(&state.pool, scope.organization_id(), project_id, false, 1).await?;
    if some.is_empty() {
        return Err(ApiError::Validation(
            "search for spots before enriching them".into(),
        ));
    }

    // Enrichment is the costliest discovery stage (2 provider calls per spot),
    // so it is metered against the same discovery quota, checked at enqueue.
    quota_gate(&state, scope.organization_id()).await?;

    let run = spots::start_enrich(&state.pool, &scope, Uuid::now_v7(), project_id).await?;
    let key = format!("enrich_spots:{}", run.id);
    let enqueued = db::repositories::jobs::enqueue(
        &state.pool,
        Some(scope.organization_id()),
        jobs::JobKind::EnrichSpots,
        serde_json::json!({ "run_id": run.id }),
        Some(&key),
    )
    .await?;
    if enqueued.is_some() {
        db::repositories::usage::add_discoveries(&state.pool, scope.organization_id(), 1).await?;
    }

    Ok((StatusCode::ACCEPTED, Json(EnrichRunView::from(run))))
}

/// The enqueue-time discovery quota check, shared by search + enrich. Resolves
/// the org's plan cap and current-period usage, then the pure
/// `backlio_core::quota::check`. Over quota → 429 `quota_exceeded`; it never
/// deletes data and never gates monitoring (§6).
async fn quota_gate(state: &AppState, organization_id: Uuid) -> Result<(), ApiError> {
    let spec = db::repositories::billing::plan_for_org(&state.pool, organization_id)
        .await?
        .spec();
    let used = db::repositories::usage::discoveries_this_period(&state.pool, organization_id)
        .await? as u32;
    let decision = backlio_core::quota::check(spec.discovery_limit, used, 1);
    if decision.allowed {
        Ok(())
    } else {
        Err(ApiError::QuotaExceeded(format!(
            "discovery quota reached for this period ({} of {} used) — it resets next month, or upgrade your plan for more.",
            decision.used,
            decision.limit.unwrap_or(0),
        )))
    }
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

    // At least one marketplace source must be configured (any provider).
    if !crawler::marketplace::any_configured() {
        return Err(ApiError::Validation(
            "no link marketplace is configured".into(),
        ));
    }

    if spots::match_in_flight(&state.pool, &scope, project_id).await?
        && let Some(existing) = spots::latest_match(&state.pool, &scope, project_id).await?
    {
        return Ok((StatusCode::ACCEPTED, Json(MatchRunView::from(existing))));
    }

    let some =
        spots::spots_to_enrich(&state.pool, scope.organization_id(), project_id, false, 1).await?;
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

#[derive(Deserialize)]
pub struct ToCampaignBody {
    pub campaign_id: Uuid,
}

/// `POST …/projects/{project_id}/spots/{spot_id}/to-campaign` — push a spot into
/// a link-building campaign as an outreach prospect. Member role; requires the
/// campaigns BETA. Carries over the spot's domain, a ranking URL, the quality
/// score and (when buyable) the marketplace price. Idempotent per campaign+domain.
pub async fn spot_to_campaign(
    State(state): State<AppState>,
    Path((org_id, _project_id, spot_id)): Path<(Uuid, Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<ToCampaignBody>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    gate(&state)?;
    if !state.cfg.campaigns_beta {
        return Err(ApiError::Validation(
            "link-building campaigns are not enabled".into(),
        ));
    }
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;

    // Both the spot and the campaign must belong to this org (404 otherwise).
    let spot = spots::get(&state.pool, &scope, spot_id).await?;
    db::repositories::campaigns::get(&state.pool, &scope, body.campaign_id).await?;

    let score = spot.quality_score.map(|q| q.round() as i32).unwrap_or(0);
    let est_cost = spot
        .buy_price_cents
        .map(|c| format!("{:.2}", (c as f64) / 100.0));
    let kw = spot.example_keyword.clone().unwrap_or_default();
    let notes = format!(
        "From Spot Finder — ranks for {} of your keyword(s){}{}",
        spot.keyword_count,
        if kw.is_empty() {
            String::new()
        } else {
            format!("; e.g. \"{kw}\"")
        },
        match &spot.marketplace {
            Some(m) => format!("; buyable on {m}"),
            None => String::new(),
        },
    );

    let prospect = db::repositories::prospects::create(
        &state.pool,
        &scope,
        Uuid::now_v7(),
        db::repositories::prospects::NewProspect {
            campaign_id: body.campaign_id,
            domain: &spot.domain,
            source: "spot_finder",
            contact_name: None,
            contact_email: None,
            target_url: spot.example_url.as_deref(),
            estimated_cost: est_cost.as_deref(),
            currency: spot.buy_currency.as_deref(),
            notes: Some(&notes),
            competitor_count: None,
            score,
            score_reason: "Spot Finder relevance + quality",
        },
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "prospect_id": prospect.id, "domain": prospect.domain })),
    ))
}

/// `POST …/projects/{project_id}/spots/discover-marketplace` — marketplace-first
/// discovery: browse the configured marketplaces' catalogues for the project's
/// kept keywords and persist the buyable sites as spots. Member role. Catalogue
/// reads don't spend provider credit, so this is not quota-gated.
pub async fn discover_marketplace_spots(
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

    if !crawler::marketplace::any_configured() {
        return Err(ApiError::Validation(
            "no link marketplace is configured".into(),
        ));
    }

    // One run at a time per project: return the in-flight one if present.
    if spots::search_in_flight(&state.pool, &scope, project_id).await?
        && let Some(existing) = spots::latest_run(&state.pool, &scope, project_id).await?
    {
        return Ok((StatusCode::ACCEPTED, Json(RunView::from(existing))));
    }

    let kept = spots::kept_keywords(&state.pool, scope.organization_id(), project_id, 1).await?;
    if kept.is_empty() {
        return Err(ApiError::Validation(
            "keep at least one keyword before discovering marketplace spots".into(),
        ));
    }

    let locale = match body.locale.as_deref() {
        Some("en") => "en",
        _ => "fr",
    };
    let run = spots::start_run(&state.pool, &scope, Uuid::now_v7(), project_id, locale).await?;
    let key = format!("discover_marketplace:{}", run.id);
    db::repositories::jobs::enqueue(
        &state.pool,
        Some(scope.organization_id()),
        jobs::JobKind::DiscoverMarketplaceSpots,
        serde_json::json!({ "run_id": run.id }),
        Some(&key),
    )
    .await?;

    Ok((StatusCode::ACCEPTED, Json(RunView::from(run))))
}

/// `POST …/projects/{project_id}/spots/{spot_id}/monitor` — the buy→monitor
/// bridge: turn a (bought) marketplace spot into a monitored backlink in one
/// click. Source = the editor site (edit to the published article URL once
/// live), target = the project's domain, cost carried from the spot's offer.
/// Member role.
pub async fn monitor_spot(
    State(state): State<AppState>,
    Path((org_id, project_id, spot_id)): Path<(Uuid, Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    gate(&state)?;
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    projects::get(&state.pool, &scope, project_id).await?;

    let spot = spots::get(&state.pool, &scope, spot_id).await?;
    let domain =
        db::repositories::projects::domain_by_id(&state.pool, scope.organization_id(), project_id)
            .await?
            .ok_or(ApiError::NotFound)?;

    // Respect the plan's monitored-link cap (like the manual add).
    crate::routes::resource_routes::enforce_backlink_quota(&state, &scope, 1).await?;

    let source_url = format!("https://{}", spot.domain);
    let target_url = format!("https://{domain}");
    let cost_str = spot
        .buy_price_cents
        .map(|c| format!("{:.2}", c as f64 / 100.0));
    let currency = spot.buy_currency.clone();

    // A purchased link has an economic reason, so it is monitored.
    let decision = backlio_core::monitoring::decide(backlio_core::MonitoringInputs {
        acquisition: None,
        has_cost: spot.buy_price_cents.unwrap_or(0) > 0,
        under_warranty: false,
        subject: "own",
        tier: 1,
    });
    let (mon_enabled, mon_level, mon_reason) =
        crate::routes::resource_routes::resolve_monitoring(&decision, Some(true));

    let new = db::repositories::backlinks::NewBacklink {
        project_id,
        campaign_id: None,
        publisher_id: None,
        source_url: source_url.as_str(),
        target_url: target_url.as_str(),
        expected_anchor_text: None,
        expected_rel_type: "dofollow",
        cost: cost_str.as_deref(),
        currency: currency.as_deref(),
        warranty_type: "none",
        warranty_value: None,
        warranty_expiration: None,
        monitoring_enabled: mon_enabled,
        monitoring_level: mon_level,
        monitoring_reason: mon_reason,
        check_frequency: None,
    };
    let backlink = db::repositories::backlinks::create(&state.pool, &scope, Uuid::now_v7(), new)
        .await
        .map_err(|e| match e {
            db::DbError::Config(_) => {
                ApiError::Validation("could not build a URL for the link".into())
            }
            other => other.into(),
        })?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "backlink_id": backlink.id,
            "source_url": source_url,
            "target_url": target_url,
        })),
    ))
}
