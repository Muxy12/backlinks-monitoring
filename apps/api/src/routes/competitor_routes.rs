//! Competitor monitoring (backlink-intelligence spec §1–§3). Competitors are
//! tracked per project; their backlinks live in the shared `backlinks` table
//! tagged `subject = competitor`, so the whole monitoring engine is reused.
//! Discovery here is the Import provider (§21): the caller supplies known
//! competitor backlinks; recrawling alone cannot discover unknown ones.

use crate::error::ApiError;
use crate::routes::resource_routes::BacklinkView;
use crate::session::{check_csrf, current};
use crate::state::AppState;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum_extra::extract::cookie::CookieJar;
use common::Role;
use db::authorize_org;
use db::repositories::{backlinks, competitors, projects};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize)]
pub struct CompetitorView {
    pub id: Uuid,
    pub project_id: Uuid,
    pub domain: String,
    pub normalized_domain: String,
    pub enabled: bool,
    pub first_scanned_at: Option<String>,
    pub last_scanned_at: Option<String>,
}

impl From<db::entities::ProjectCompetitor> for CompetitorView {
    fn from(c: db::entities::ProjectCompetitor) -> Self {
        CompetitorView {
            id: c.id,
            project_id: c.project_id,
            domain: c.domain,
            normalized_domain: c.normalized_domain,
            enabled: c.enabled,
            first_scanned_at: c.first_scanned_at.map(|d| d.to_rfc3339()),
            last_scanned_at: c.last_scanned_at.map(|d| d.to_rfc3339()),
        }
    }
}

#[derive(Deserialize)]
pub struct CreateCompetitorBody {
    pub domain: String,
}

pub async fn create_competitor(
    State(state): State<AppState>,
    Path((org_id, project_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<CreateCompetitorBody>,
) -> Result<(StatusCode, Json<CompetitorView>), ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    // The project must belong to this org (404 otherwise) so a cross-tenant
    // project_id can never be stitched onto a competitor row.
    projects::get(&state.pool, &scope, project_id).await?;
    let domain = body.domain.trim();
    if domain.is_empty() {
        return Err(ApiError::Validation("domain is required".into()));
    }
    let c = competitors::create(&state.pool, &scope, Uuid::now_v7(), project_id, domain).await?;
    Ok((StatusCode::CREATED, Json(c.into())))
}

pub async fn list_competitors(
    State(state): State<AppState>,
    Path((org_id, project_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
) -> Result<Json<Vec<CompetitorView>>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Viewer)?;
    let rows = competitors::list(&state.pool, &scope, project_id).await?;
    Ok(Json(rows.into_iter().map(CompetitorView::from).collect()))
}

pub async fn delete_competitor(
    State(state): State<AppState>,
    Path((org_id, competitor_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    if competitors::delete(&state.pool, &scope, competitor_id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound)
    }
}

pub async fn list_competitor_backlinks(
    State(state): State<AppState>,
    Path((org_id, competitor_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
) -> Result<Json<Vec<BacklinkView>>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Viewer)?;
    // 404 if the competitor is not in this org (tenant isolation).
    competitors::get(&state.pool, &scope, competitor_id).await?;
    let rows = backlinks::search_competitor(&state.pool, &scope, competitor_id, 500).await?;
    Ok(Json(rows.into_iter().map(BacklinkView::from).collect()))
}

#[derive(Deserialize)]
pub struct GapQuery {
    pub min: Option<i64>,
}

#[derive(Serialize)]
pub struct GapRow {
    pub source_domain: String,
    pub competitor_count: i64,
}

/// Backlink gap (§4): domains linking to ≥ `min` competitors but not to us.
pub async fn backlink_gap(
    State(state): State<AppState>,
    Path((org_id, project_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    axum::extract::Query(q): axum::extract::Query<GapQuery>,
) -> Result<Json<Vec<GapRow>>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Viewer)?;
    let rows =
        competitors::gap_analysis(&state.pool, &scope, project_id, q.min.unwrap_or(1), 500).await?;
    Ok(Json(
        rows.into_iter()
            .map(|(source_domain, competitor_count)| GapRow {
                source_domain,
                competitor_count,
            })
            .collect(),
    ))
}

#[derive(Deserialize)]
pub struct ImportRow {
    pub source_url: String,
    pub target_url: String,
    pub anchor: Option<String>,
    pub rel: Option<String>,
}

#[derive(Deserialize)]
pub struct ImportBody {
    #[serde(default)]
    pub rows: Vec<ImportRow>,
    /// Alternatively, a raw CSV export (Majestic / Ahrefs / Semrush / generic),
    /// parsed server-side with the shared column-mapping parser.
    #[serde(default)]
    pub csv: Option<String>,
}

#[derive(Serialize)]
pub struct ImportResult {
    pub imported: usize,
    pub new: usize,
    pub skipped: usize,
}

fn is_http(s: &str) -> bool {
    let s = s.trim();
    s.starts_with("http://") || s.starts_with("https://")
}

/// Import provider (§21): ingest known competitor backlinks. New relations emit
/// competitor_new_backlink / _referring_page / _referring_domain events.
pub async fn import_competitor_backlinks(
    State(state): State<AppState>,
    Path((org_id, competitor_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<ImportBody>,
) -> Result<Json<ImportResult>, ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    let competitor = competitors::get(&state.pool, &scope, competitor_id).await?;

    // Accept either a raw CSV export (parsed with the shared column mapper, so
    // Majestic / Ahrefs / Semrush / generic headers all work) or pre-structured
    // rows.
    let rows: Vec<(String, String, Option<String>, String)> =
        if let Some(csv) = body.csv.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            let parsed = backlio_core::csv_import::parse_csv(csv).map_err(ApiError::Validation)?;
            parsed
                .valid()
                .map(|r| {
                    (
                        r.source_url.clone(),
                        r.target_url.clone(),
                        r.anchor.clone(),
                        r.rel_type.clone(),
                    )
                })
                .collect()
        } else {
            body.rows
                .into_iter()
                .map(|row| {
                    (
                        row.source_url,
                        row.target_url,
                        row.anchor,
                        row.rel.unwrap_or_else(|| "dofollow".into()),
                    )
                })
                .collect()
        };

    let mut imported = 0usize;
    let mut new = 0usize;
    let mut skipped = 0usize;
    for (source_url, target_url, anchor, rel) in rows.into_iter().take(5_000) {
        if !is_http(&source_url)
            || !is_http(&target_url)
            || !matches!(rel.as_str(), "dofollow" | "nofollow" | "sponsored" | "ugc")
        {
            skipped += 1;
            continue;
        }
        match competitors::record_backlink(
            &state.pool,
            &scope,
            &competitor,
            source_url.trim(),
            target_url.trim(),
            anchor.as_deref().map(str::trim).filter(|s| !s.is_empty()),
            &rel,
        )
        .await
        {
            Ok((_, is_new)) => {
                imported += 1;
                if is_new {
                    new += 1;
                }
            }
            Err(_) => skipped += 1,
        }
    }
    Ok(Json(ImportResult {
        imported,
        new,
        skipped,
    }))
}
