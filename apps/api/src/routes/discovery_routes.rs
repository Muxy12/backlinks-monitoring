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
use chrono::{DateTime, Utc};
use common::Role;
use db::authorize_org;
use db::repositories::{competitors, projects, seeds};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Parse a provider timestamp (RFC3339, or DataForSEO's "YYYY-MM-DD HH:MM:SS +ZZ:ZZ").
fn parse_dt(s: &Option<String>) -> Option<DateTime<Utc>> {
    let s = s.as_deref()?.trim();
    if let Ok(d) = DateTime::parse_from_rfc3339(s) {
        return Some(d.with_timezone(&Utc));
    }
    DateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S %z")
        .ok()
        .map(|d| d.with_timezone(&Utc))
}

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
    // The project must belong to this org (404 otherwise) so a cross-tenant
    // project_id can never be used to attach imported seeds.
    projects::get(&state.pool, &scope, project_id).await?;
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

#[derive(Deserialize)]
pub struct RunDiscoveryBody {
    pub provider: String,
    /// When set, discover for this competitor; otherwise for the project's site.
    pub competitor_id: Option<Uuid>,
    pub limit: Option<usize>,
}

#[derive(Serialize)]
pub struct RunDiscoveryResult {
    pub provider: String,
    pub discovered: usize,
    pub new: usize,
}

/// Run an external discovery provider (spec §21). Providers are gated by env
/// secrets — an unconfigured provider returns 400 rather than calling anything.
pub async fn run_discovery(
    State(state): State<AppState>,
    Path((org_id, project_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<RunDiscoveryBody>,
) -> Result<Json<RunDiscoveryResult>, ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;

    let provider = crawler::discovery::from_env(&body.provider).ok_or_else(|| {
        ApiError::Validation(format!(
            "discovery provider '{}' is not configured (set its API secrets)",
            body.provider
        ))
    })?;
    let limit = body.limit.unwrap_or(200).min(1000);

    // Target: a competitor's domain, or the project's own domain.
    let competitor = match body.competitor_id {
        Some(cid) => Some(competitors::get(&state.pool, &scope, cid).await?),
        None => None,
    };
    let target_domain = match &competitor {
        Some(c) => c.normalized_domain.clone(),
        None => projects::get(&state.pool, &scope, project_id).await?.domain,
    };

    let links = provider
        .discover(&target_domain, limit)
        .await
        .map_err(|e| {
            // Surface the provider's own message (bad key, no API access, no funds,
            // invalid target) to the operator instead of a generic 500 — this is an
            // authenticated action on the caller's own org/keys.
            tracing::warn!(provider = %body.provider, error = %e, "discovery provider failed");
            ApiError::Validation(format!("{} discovery failed: {e}", body.provider))
        })?;
    let discovered = links.len();
    let mut new = 0usize;
    for l in links {
        let rel = if l.dofollow { "dofollow" } else { "nofollow" };
        let meta = db::repositories::DiscoveredMeta {
            first_seen: parse_dt(&l.first_seen),
            last_seen: parse_dt(&l.last_seen),
            source_status: l.source_status,
        };
        let is_new = match &competitor {
            Some(c) => competitors::record_backlink_with_meta(
                &state.pool,
                &scope,
                c,
                &l.source_url,
                &l.target_url,
                l.anchor.as_deref(),
                rel,
                Some(&meta),
            )
            .await
            .map(|(_, n)| n)
            .unwrap_or(false),
            None => seeds::record_backlink_with_meta(
                &state.pool,
                org_id,
                project_id,
                &l.source_url,
                &l.target_url,
                l.anchor.as_deref(),
                rel,
                Some(&meta),
            )
            .await
            .unwrap_or(false),
        };
        if is_new {
            new += 1;
        }
    }
    // Own-domain discovery feeds the project's inventory: flag footprints on the
    // newly discovered links so the "Likely low-value" filter reflects them
    // without a manual Detect footprints (best-effort; never fails the scan).
    if competitor.is_none()
        && new > 0
        && let Err(e) =
            crate::routes::resource_routes::autotag_project_footprints(&state, &scope, project_id)
                .await
    {
        tracing::warn!(error = %e, "auto-tag after discovery failed");
    }

    Ok(Json(RunDiscoveryResult {
        provider: body.provider,
        discovered,
        new,
    }))
}
