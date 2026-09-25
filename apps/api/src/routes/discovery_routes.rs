//! Discovery seeds (spec §21–§22). Import referring domains (e.g. from a GSC
//! "Top linking sites" export); the expansion crawler later visits each to turn
//! domain-level hints into real source→target backlinks.

use crate::error::ApiError;
use crate::session::{check_csrf, current};
use crate::state::AppState;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum_extra::extract::cookie::CookieJar;
use common::Role;
use db::authorize_org;
use db::repositories::seeds;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize)]
pub struct SeedView {
    pub id: Uuid,
    pub domain: String,
    pub normalized_domain: String,
    pub source: String,
    pub status: String,
    pub pages_crawled: i32,
    pub backlinks_found: i32,
    pub last_crawled_at: Option<String>,
    pub next_crawl_at: String,
}

impl From<db::entities::DiscoverySeed> for SeedView {
    fn from(s: db::entities::DiscoverySeed) -> Self {
        SeedView {
            id: s.id,
            domain: s.domain,
            normalized_domain: s.normalized_domain,
            source: s.source,
            status: s.status,
            pages_crawled: s.pages_crawled,
            backlinks_found: s.backlinks_found,
            last_crawled_at: s.last_crawled_at.map(|d| d.to_rfc3339()),
            next_crawl_at: s.next_crawl_at.to_rfc3339(),
        }
    }
}

pub async fn list_seeds(
    State(state): State<AppState>,
    Path((org_id, project_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
) -> Result<Json<Vec<SeedView>>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Viewer)?;
    let rows = seeds::list(&state.pool, &scope, project_id).await?;
    Ok(Json(rows.into_iter().map(SeedView::from).collect()))
}

#[derive(Deserialize)]
pub struct ImportSeedsBody {
    pub domains: Vec<String>,
    pub source: Option<String>,
}

#[derive(Serialize)]
pub struct ImportSeedsResult {
    pub added: usize,
    pub skipped: usize,
}

pub async fn import_seeds(
    State(state): State<AppState>,
    Path((org_id, project_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<ImportSeedsBody>,
) -> Result<Json<ImportSeedsResult>, ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    let source = match body.source.as_deref() {
        Some("gsc") => "gsc",
        Some("manual") => "manual",
        _ => "import",
    };
    let (mut added, mut skipped) = (0usize, 0usize);
    for domain in body.domains.into_iter().take(10_000) {
        let d = domain.trim();
        if d.is_empty() {
            skipped += 1;
            continue;
        }
        match seeds::create(&state.pool, &scope, Uuid::now_v7(), project_id, d, source).await {
            Ok(Some(_)) => added += 1,
            Ok(None) => skipped += 1, // duplicate or invalid domain
            Err(_) => skipped += 1,
        }
    }
    Ok(Json(ImportSeedsResult { added, skipped }))
}
