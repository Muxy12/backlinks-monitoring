//! Spot Finder — keyword detection & review (Lot A).
//!
//! The prospecting engine (find sites that rank for the user's keywords, score
//! them, cross-reference link-buying marketplaces) starts from keywords. Rather
//! than ask the user to type them, we crawl their OWN site (homepage + pages one
//! click away) and extract the `<h1>` headings as candidates, which the user
//! then reviews/edits/prunes before any paid SERP lookup. Detection runs as a
//! background job (`detect_site_keywords`); these routes start it, report its
//! status, and manage the reviewable list.
//!
//! Env-gated BETA: when `spot_finder` is off every route 404s and the UI hides
//! the surface (surfaced via `me.features.spot_finder`).

use crate::error::ApiError;
use crate::routes::resource_routes::enforce_active_access;
use crate::session::{check_csrf, current};
use crate::state::{ApiConfig, AppState};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum_extra::extract::cookie::CookieJar;
use common::Role;
use db::authorize_org;
use db::repositories::{projects, spot_keywords, website_analyses};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Whether the Spot Finder BETA is enabled for this deployment.
pub fn configured(cfg: &ApiConfig) -> bool {
    cfg.spot_finder
}

/// 404 (feature hidden) unless Spot Finder is enabled.
fn gate(state: &AppState) -> Result<(), ApiError> {
    if configured(&state.cfg) {
        Ok(())
    } else {
        Err(ApiError::NotFound)
    }
}

fn valid_status(s: &str) -> bool {
    matches!(s, "kept" | "rejected" | "candidate")
}

#[derive(Serialize)]
pub struct ScanView {
    pub id: Uuid,
    pub status: String,
    pub pages_scanned: i32,
    pub h1_count: i32,
    pub keywords_found: i32,
    pub error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<db::entities::SpotKeywordScan> for ScanView {
    fn from(s: db::entities::SpotKeywordScan) -> Self {
        ScanView {
            id: s.id,
            status: s.status,
            pages_scanned: s.pages_scanned,
            h1_count: s.h1_count,
            keywords_found: s.keywords_found,
            error: s.error,
            created_at: s.created_at.to_rfc3339(),
            updated_at: s.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Serialize)]
pub struct KeywordView {
    pub id: Uuid,
    pub keyword: String,
    pub source: String,
    pub origin_url: Option<String>,
    pub status: String,
    pub created_at: String,
}

impl From<db::entities::SpotKeyword> for KeywordView {
    fn from(k: db::entities::SpotKeyword) -> Self {
        KeywordView {
            id: k.id,
            keyword: k.keyword,
            source: k.source,
            origin_url: k.origin_url,
            status: k.status,
            created_at: k.created_at.to_rfc3339(),
        }
    }
}

#[derive(Serialize)]
pub struct KeywordsResponse {
    /// The most recent detection run, so the UI can show queued/running/done.
    pub scan: Option<ScanView>,
    pub keywords: Vec<KeywordView>,
}

/// `GET …/projects/{project_id}/keywords` — the reviewable keyword list plus the
/// latest detection-run status. Any member may read.
pub async fn list_keywords(
    State(state): State<AppState>,
    Path((org_id, project_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
) -> Result<Json<KeywordsResponse>, ApiError> {
    gate(&state)?;
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Viewer)?;
    // Project must belong to this org (404 otherwise).
    projects::get(&state.pool, &scope, project_id).await?;

    let scan = spot_keywords::latest_scan(&state.pool, &scope, project_id).await?;
    let keywords = spot_keywords::list(&state.pool, &scope, project_id).await?;
    Ok(Json(KeywordsResponse {
        scan: scan.map(ScanView::from),
        keywords: keywords.into_iter().map(KeywordView::from).collect(),
    }))
}

/// `POST …/projects/{project_id}/keywords/detect` — start a keyword-detection
/// scan of the project's own site. Returns the scan (queued or the one already
/// running). Member role. No external cost: it crawls the user's own site.
pub async fn detect_keywords(
    State(state): State<AppState>,
    Path((org_id, project_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
) -> Result<(StatusCode, Json<ScanView>), ApiError> {
    gate(&state)?;
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    projects::get(&state.pool, &scope, project_id).await?;

    // One scan at a time per project: if one is already queued/running, return it
    // instead of starting a second crawl.
    if spot_keywords::scan_in_flight(&state.pool, &scope, project_id).await?
        && let Some(existing) = spot_keywords::latest_scan(&state.pool, &scope, project_id).await?
    {
        return Ok((StatusCode::ACCEPTED, Json(ScanView::from(existing))));
    }

    let scan = spot_keywords::start_scan(&state.pool, &scope, Uuid::now_v7(), project_id).await?;
    // Idempotency key is per-scan, so it never dedupes a distinct scan — it only
    // guards against this very scan being enqueued twice.
    let key = format!("detect_site_keywords:{}", scan.id);
    db::repositories::jobs::enqueue(
        &state.pool,
        Some(scope.organization_id()),
        jobs::JobKind::DetectSiteKeywords,
        serde_json::json!({ "scan_id": scan.id }),
        Some(&key),
    )
    .await?;

    Ok((StatusCode::ACCEPTED, Json(ScanView::from(scan))))
}

#[derive(Deserialize)]
pub struct AnalyzeBody {
    /// A site URL or bare domain, e.g. `monsite.com` or `https://monsite.com/x`.
    pub url: String,
}

#[derive(Serialize)]
pub struct AnalyzeView {
    pub project_id: Uuid,
    /// True when a new project was created for this domain (vs. reusing one).
    pub created: bool,
    pub scan: ScanView,
}

/// `POST …/organizations/{org_id}/analyze` — the URL-first front door
/// (Référencime-style "new campaign"). Resolves (or creates) the project for the
/// site's domain, then starts a keyword-detection scan of it. The client polls
/// the scan, auto-keeps the detected topics and triggers marketplace discovery —
/// URL in, scored spots out. Member role; detection is free (crawls own site).
pub async fn analyze_url(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<AnalyzeBody>,
) -> Result<(StatusCode, Json<AnalyzeView>), ApiError> {
    gate(&state)?;
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;

    // Accept a bare domain by giving host parsing a scheme to work with.
    let raw = body.url.trim();
    if raw.is_empty() {
        return Err(ApiError::Validation("enter a site URL".into()));
    }
    let with_scheme = if raw.contains("://") {
        raw.to_string()
    } else {
        format!("https://{raw}")
    };
    let host = backlio_core::host_of(&with_scheme)
        .filter(|h| h.contains('.'))
        .ok_or_else(|| ApiError::Validation("enter a valid site URL".into()))?;

    // Resolve-or-create the project for this domain, so re-analyzing reuses it.
    let (project, created) = match projects::find_by_domain(&state.pool, &scope, &host).await? {
        Some(p) => (p, false),
        None => {
            enforce_active_access(&state, &scope).await?;
            let plan =
                db::repositories::billing::plan_for_org(&state.pool, scope.organization_id())
                    .await?;
            let current = projects::count(&state.pool, &scope).await? as u32;
            if let billing::AddOutcome::OverLimit { limit, .. } =
                billing::Entitlements::new(plan).can_add_project(current)
            {
                return Err(ApiError::PlanLimit(format!(
                    "plan limit of {limit} projects reached — upgrade to analyze another site"
                )));
            }
            let p = projects::create(
                &state.pool,
                &scope,
                Uuid::now_v7(),
                &host,
                &host,
                24 * 7 * 3600,
                None,
            )
            .await?;
            (p, true)
        }
    };

    // Start (or reuse) a detection scan of the site — one at a time per project.
    let scan = if spot_keywords::scan_in_flight(&state.pool, &scope, project.id).await?
        && let Some(existing) = spot_keywords::latest_scan(&state.pool, &scope, project.id).await?
    {
        existing
    } else {
        let s = spot_keywords::start_scan(&state.pool, &scope, Uuid::now_v7(), project.id).await?;
        let key = format!("detect_site_keywords:{}", s.id);
        db::repositories::jobs::enqueue(
            &state.pool,
            Some(scope.organization_id()),
            jobs::JobKind::DetectSiteKeywords,
            serde_json::json!({ "scan_id": s.id }),
            Some(&key),
        )
        .await?;
        s
    };

    Ok((
        StatusCode::ACCEPTED,
        Json(AnalyzeView {
            project_id: project.id,
            created,
            scan: ScanView::from(scan),
        }),
    ))
}

#[derive(Serialize)]
pub struct AnalysisView {
    pub status: String,
    pub source: Option<String>,
    pub language: Option<String>,
    pub theme: Option<String>,
    pub sub_themes: Vec<String>,
    pub products: Vec<String>,
    /// Detected keywords (AI/heuristic) — NOT verified Google positions.
    pub keywords: Vec<String>,
    pub summary: Option<String>,
    pub edited: bool,
    pub error: Option<String>,
    pub updated_at: String,
}

impl From<website_analyses::WebsiteAnalysis> for AnalysisView {
    fn from(a: website_analyses::WebsiteAnalysis) -> Self {
        AnalysisView {
            status: a.status,
            source: a.source,
            language: a.language,
            theme: a.theme,
            sub_themes: a.sub_themes,
            products: a.products,
            keywords: a.keywords,
            summary: a.summary,
            edited: a.edited,
            error: a.error,
            updated_at: a.updated_at.to_rfc3339(),
        }
    }
}

/// `POST …/projects/{project_id}/analyze-site` — (re)build the site's AI/heuristic
/// semantic profile. Member role; free (crawls the user's own site).
pub async fn analyze_site(
    State(state): State<AppState>,
    Path((org_id, project_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
) -> Result<(StatusCode, Json<AnalysisView>), ApiError> {
    gate(&state)?;
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    projects::get(&state.pool, &scope, project_id).await?;

    let analysis = website_analyses::upsert_queued(&state.pool, &scope, project_id).await?;
    let key = format!("analyze_site:{project_id}");
    db::repositories::jobs::enqueue(
        &state.pool,
        Some(scope.organization_id()),
        jobs::JobKind::AnalyzeSite,
        serde_json::json!({ "org_id": org_id, "project_id": project_id }),
        Some(&key),
    )
    .await?;
    Ok((StatusCode::ACCEPTED, Json(AnalysisView::from(analysis))))
}

/// `GET …/projects/{project_id}/analysis` — the site's current analysis, or null
/// if never analysed. Any member may read.
pub async fn get_analysis(
    State(state): State<AppState>,
    Path((org_id, project_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
) -> Result<Json<Option<AnalysisView>>, ApiError> {
    gate(&state)?;
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Viewer)?;
    projects::get(&state.pool, &scope, project_id).await?;
    let a = website_analyses::get_by_project(&state.pool, &scope, project_id).await?;
    Ok(Json(a.map(AnalysisView::from)))
}

#[derive(Deserialize)]
pub struct AnalysisEditBody {
    pub theme: Option<String>,
    #[serde(default)]
    pub sub_themes: Vec<String>,
    #[serde(default)]
    pub products: Vec<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
    pub summary: Option<String>,
}

/// `PATCH …/projects/{project_id}/analysis` — save the user's corrections to the
/// detected profile. Member role; `NotFound` if the site was never analysed.
pub async fn update_analysis(
    State(state): State<AppState>,
    Path((org_id, project_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<AnalysisEditBody>,
) -> Result<Json<AnalysisView>, ApiError> {
    gate(&state)?;
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    projects::get(&state.pool, &scope, project_id).await?;
    let a = website_analyses::set_edits(
        &state.pool,
        &scope,
        project_id,
        body.theme.as_deref(),
        &body.sub_themes,
        &body.products,
        &body.keywords,
        body.summary.as_deref(),
    )
    .await?
    .ok_or(ApiError::NotFound)?;
    Ok(Json(AnalysisView::from(a)))
}

#[derive(Deserialize)]
pub struct AddKeywordBody {
    pub keyword: String,
}

/// `POST …/projects/{project_id}/keywords` — add a keyword by hand (status
/// `kept`). Member role.
pub async fn add_keyword(
    State(state): State<AppState>,
    Path((org_id, project_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<AddKeywordBody>,
) -> Result<(StatusCode, Json<KeywordView>), ApiError> {
    gate(&state)?;
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    projects::get(&state.pool, &scope, project_id).await?;

    let cleaned = backlio_core::clean_keyword(&body.keyword)
        .ok_or_else(|| ApiError::Validation("that keyword is empty or too long".into()))?;
    let row = spot_keywords::add_manual(
        &state.pool,
        &scope,
        Uuid::now_v7(),
        project_id,
        &cleaned.display,
        &cleaned.normalized,
    )
    .await?
    .ok_or_else(|| ApiError::Validation("that keyword already exists for this project".into()))?;
    Ok((StatusCode::CREATED, Json(KeywordView::from(row))))
}

#[derive(Deserialize)]
pub struct UpdateKeywordBody {
    /// New review state: kept | rejected | candidate.
    pub status: Option<String>,
    /// New display text (re-normalized server-side).
    pub keyword: Option<String>,
}

/// `PATCH …/projects/{project_id}/keywords/{keyword_id}` — edit a keyword's text
/// and/or review status. Member role.
pub async fn update_keyword(
    State(state): State<AppState>,
    Path((org_id, _project_id, keyword_id)): Path<(Uuid, Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<UpdateKeywordBody>,
) -> Result<Json<KeywordView>, ApiError> {
    gate(&state)?;
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;

    if body.status.is_none() && body.keyword.is_none() {
        return Err(ApiError::Validation("nothing to update".into()));
    }

    // Edit text first (if any), then status, returning the final row.
    let mut current_row = None;
    if let Some(kw) = body.keyword.as_deref() {
        let cleaned = backlio_core::clean_keyword(kw)
            .ok_or_else(|| ApiError::Validation("that keyword is empty or too long".into()))?;
        current_row = Some(
            spot_keywords::update_text(
                &state.pool,
                &scope,
                keyword_id,
                &cleaned.display,
                &cleaned.normalized,
            )
            .await?,
        );
    }
    if let Some(status) = body.status.as_deref() {
        if !valid_status(status) {
            return Err(ApiError::Validation(
                "status must be kept, rejected or candidate".into(),
            ));
        }
        current_row =
            Some(spot_keywords::set_status(&state.pool, &scope, keyword_id, status).await?);
    }

    Ok(Json(KeywordView::from(
        current_row.expect("one field was set"),
    )))
}

#[derive(Deserialize)]
pub struct BulkStatusBody {
    pub ids: Vec<Uuid>,
    pub status: String,
}

#[derive(Serialize)]
pub struct BulkStatusResult {
    pub updated: u64,
}

/// `POST …/projects/{project_id}/keywords/bulk-status` — keep/reject many at
/// once. Member role.
pub async fn bulk_status(
    State(state): State<AppState>,
    Path((org_id, _project_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<BulkStatusBody>,
) -> Result<Json<BulkStatusResult>, ApiError> {
    gate(&state)?;
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;

    if !valid_status(&body.status) {
        return Err(ApiError::Validation(
            "status must be kept, rejected or candidate".into(),
        ));
    }
    let updated =
        spot_keywords::set_status_many(&state.pool, &scope, &body.ids, &body.status).await?;
    Ok(Json(BulkStatusResult { updated }))
}

/// `DELETE …/projects/{project_id}/keywords/{keyword_id}` — remove a keyword.
/// Member role. 204 on success, 404 if not in scope.
pub async fn delete_keyword(
    State(state): State<AppState>,
    Path((org_id, _project_id, keyword_id)): Path<(Uuid, Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    gate(&state)?;
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    spot_keywords::delete(&state.pool, &scope, keyword_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
