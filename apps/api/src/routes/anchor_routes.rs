//! Anchor text distribution for a project. ONE read endpoint: load every own
//! backlink's anchor from the DB, classify + aggregate with the pure core
//! (`backlio_core::anchors`), return the mix. No external calls, no score —
//! an observed distribution plus repeated anchors and, when one keyword phrase
//! dominates, an observation (never a verdict; §27).

use crate::error::ApiError;
use crate::session::current;
use crate::state::AppState;
use axum::Json;
use axum::extract::{Path, State};
use axum_extra::extract::cookie::CookieJar;
use backlio_core::anchors::{BrandTokens, TopAnchor, distribution};
use db::authorize_org;
use db::repositories::{anchors as anchors_repo, projects};
use serde::Serialize;
use uuid::Uuid;

/// How many repeated anchors to return.
const TOP_N: usize = 12;

/// The honest caveat shown next to the chart: we classify, we do not score, and
/// we cannot compute true "exact-match" without the page's target keyword.
const NOTE: &str = "Observed from the anchors we've seen on your live links. \
     Categories describe the mix — LinkPulz never turns this into a toxic score. \
     \"Exact-match\" isn't shown because it needs each page's target keyword, \
     which we don't store; the keyword bucket plus the most-repeated anchors are \
     the honest, actionable signal.";

#[derive(Serialize)]
pub struct AnchorsView {
    /// Live backlinks counted (including text-less / image links).
    pub total: usize,
    /// Distinct non-empty anchor phrases.
    pub distinct: usize,
    pub categories: Vec<CategoryOut>,
    pub top_anchors: Vec<AnchorOut>,
    pub over_concentration: Option<AnchorOut>,
    pub note: &'static str,
}

#[derive(Serialize)]
pub struct CategoryOut {
    pub slug: &'static str,
    pub label: &'static str,
    pub count: usize,
    pub percent: f64,
}

#[derive(Serialize)]
pub struct AnchorOut {
    pub anchor: String,
    pub slug: &'static str,
    pub label: &'static str,
    pub count: usize,
    pub percent: f64,
}

fn anchor_out(a: &TopAnchor) -> AnchorOut {
    AnchorOut {
        anchor: a.anchor.clone(),
        slug: a.kind.slug(),
        label: a.kind.label(),
        count: a.count,
        percent: round1(a.percent),
    }
}

/// One decimal place — the UI shows whole-ish percentages, not float noise.
fn round1(p: f64) -> f64 {
    (p * 10.0).round() / 10.0
}

pub async fn get_anchors(
    State(state): State<AppState>,
    Path((org_id, project_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
) -> Result<Json<AnchorsView>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    // 404 if the project isn't in this tenant.
    let project = projects::get(&state.pool, &scope, project_id).await?;

    let raw = anchors_repo::list_for_project(&state.pool, &scope, project_id).await?;
    let brand = BrandTokens::from_project(&project.domain, &project.name);
    let dist = distribution(&raw, &brand, TOP_N);

    // `distribution` already returns every category in display order.
    let categories = dist
        .categories
        .iter()
        .map(|c| CategoryOut {
            slug: c.kind.slug(),
            label: c.kind.label(),
            count: c.count,
            percent: round1(c.percent),
        })
        .collect();

    Ok(Json(AnchorsView {
        total: dist.total,
        distinct: dist.distinct,
        categories,
        top_anchors: dist.top_anchors.iter().map(anchor_out).collect(),
        over_concentration: dist.over_concentration.as_ref().map(anchor_out),
        note: NOTE,
    }))
}
