//! LinkAvista marketplace integration (Lot 1: read-only catalogue browse).
//!
//! The API key is a per-deployment secret held in `ApiConfig` and never leaves
//! the server; these handlers proxy the browsable advertiser catalogue to the
//! UI. When unconfigured the feature is simply off (routes 404, like GSC),
//! surfaced via `me.features.linkavista`. No orders and no spend here — ordering
//! is a later increment and will be sandbox-first and explicitly confirmed.

use crate::error::ApiError;
use crate::session::current;
use crate::state::{ApiConfig, AppState};
use axum::Json;
use axum::extract::{Path, Query, State};
use axum_extra::extract::cookie::CookieJar;
use common::Role;
use crawler::linkavista::{self, SiteQuery};
use db::authorize_org;
use serde::Deserialize;
use uuid::Uuid;

/// Whether the LinkAvista integration is configured (API key present).
pub fn configured(cfg: &ApiConfig) -> bool {
    cfg.linkavista_api_key.is_some()
}

#[derive(Debug, Deserialize)]
pub struct SiteSearchParams {
    pub da_min: Option<u32>,
    pub tf_min: Option<u32>,
    /// Max price in EUR cents, excl. VAT.
    pub price_max: Option<u64>,
    pub language: Option<String>,
    pub q: Option<String>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

/// `GET /v1/organizations/{org_id}/linkavista/sites` — browse the LinkAvista
/// advertiser catalogue. Tenant-scoped (any member can browse). Read-only: no
/// orders, no spend. 404 when the integration isn't configured.
pub async fn list_sites(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
    Query(params): Query<SiteSearchParams>,
) -> Result<Json<linkavista::SitesPage>, ApiError> {
    // Config gate first (matches GSC): unconfigured -> 404, feature hidden.
    let client = linkavista::from_cfg(
        state.cfg.linkavista_api_key.as_deref(),
        state.cfg.linkavista_base_url.as_deref(),
    )
    .ok_or(ApiError::NotFound)?;

    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;

    let q = SiteQuery {
        da_min: params.da_min,
        tf_min: params.tf_min,
        price_max_cents: params.price_max,
        language: params.language,
        search: params.q,
        page: params.page,
        per_page: params.per_page,
    };
    let page = client.list_sites(&q).await.map_err(|e| {
        tracing::warn!(error = %e, "linkavista catalogue fetch failed");
        ApiError::Validation(format!("linkavista: {e}"))
    })?;
    Ok(Json(page))
}
