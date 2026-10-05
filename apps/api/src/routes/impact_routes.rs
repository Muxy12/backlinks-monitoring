//! Post-Link Impact (BETA): observe what happened to a backlink's TARGET page
//! after acquisition. ONE read endpoint composes the layers from locally stored
//! `gsc_daily` — **no external calls on open** (§69). Correlation, not causation:
//! every field is an observed before/after, never "this link caused…".
//!
//! Crawl + Indexation are "unavailable" in this BETA (they need URL Inspection /
//! logs). Visibility + Ranking come from Search Analytics via `gsc_daily`.

use crate::error::ApiError;
use crate::session::{check_csrf, current};
use crate::state::AppState;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum_extra::extract::cookie::CookieJar;
use backlio_core::impact::{
    POSITION_THRESHOLD, RATE_THRESHOLD_PCT, SignalState, change_percent, confidence, headline,
    per_day, position_signal, up_signal,
};
use chrono::{DateTime, Duration, Utc};
use common::Role;
use db::authorize_org;
use db::entities::Backlink;
use db::repositories::gsc_daily::WindowAgg;
use db::repositories::{backlinks, events, gsc, gsc_daily, impact};
use serde::Serialize;
use uuid::Uuid;

const WINDOWS: &[i64] = &[7, 28];

// ─── Response shape ───────────────────────────────────────────────────────────
#[derive(Serialize)]
pub struct ImpactView {
    pub status: String,
    pub eligible: bool,
    pub impact_started_at: Option<String>,
    pub t0_source: Option<String>,
    pub data_sources: Vec<&'static str>,
    pub crawl: LayerStub,
    pub indexation: LayerStub,
    pub visibility: VisibilityLayer,
    pub ranking: RankingLayer,
    pub timeline: Vec<TimelineItem>,
    pub context: Context,
    pub confidence: Option<String>,
    pub summary: String,
}

#[derive(Serialize)]
pub struct LayerStub {
    pub status: String,
    pub note: Option<String>,
}

#[derive(Serialize)]
pub struct VisibilityLayer {
    pub status: String,
    pub windows: Vec<VisibilityWindow>,
}

#[derive(Serialize)]
pub struct VisibilityWindow {
    pub label: String,
    pub status: String,
    pub available_on: Option<String>,
    pub impressions_before: i64,
    pub impressions_after: i64,
    pub clicks_before: i64,
    pub clicks_after: i64,
    pub change_percent: Option<f64>,
}

#[derive(Serialize)]
pub struct RankingLayer {
    pub status: String,
    pub position_before: Option<f64>,
    pub position_after: Option<f64>,
    pub position_change: Option<f64>,
}

#[derive(Serialize)]
pub struct TimelineItem {
    pub kind: String,
    pub at: String,
    pub label: String,
}

#[derive(Serialize)]
pub struct Context {
    pub overlapping_acquisitions: i64,
}

// ─── Helpers ──────────────────────────────────────────────────────────────────
fn resolve_t0(b: &Backlink) -> (DateTime<Utc>, &'static str) {
    if let Some(pd) = b.purchase_date {
        (pd.and_hms_opt(0, 0, 0).unwrap().and_utc(), "acquisition")
    } else if let Some(fc) = b.first_checked_at {
        (fc, "verification")
    } else {
        (b.created_at, "acquisition")
    }
}

fn eligibility(b: &Backlink) -> Option<&'static str> {
    if b.monitoring_level == "protected" {
        Some("protected")
    } else if b.monitoring_enabled {
        Some("monitored")
    } else if b.cost.is_some() {
        Some("has_cost")
    } else if b.campaign_id.is_some() {
        Some("campaign")
    } else {
        None
    }
}

fn avg_pos(w: &WindowAgg) -> Option<f64> {
    if w.pos_weight > 0 {
        Some(w.pos_weighted / w.pos_weight as f64)
    } else {
        None
    }
}

fn unavailable_layer(note: &str) -> LayerStub {
    LayerStub {
        status: SignalState::Unavailable.as_str().to_string(),
        note: Some(note.to_string()),
    }
}

/// The view for a backlink that isn't eligible (and has no manual analysis).
fn not_eligible_view() -> ImpactView {
    ImpactView {
        status: SignalState::Unavailable.as_str().to_string(),
        eligible: false,
        impact_started_at: None,
        t0_source: None,
        data_sources: vec![],
        crawl: unavailable_layer("Crawl observation isn't available yet."),
        indexation: unavailable_layer("Indexation data isn't available yet."),
        visibility: VisibilityLayer {
            status: SignalState::Unavailable.as_str().to_string(),
            windows: vec![],
        },
        ranking: RankingLayer {
            status: SignalState::Unavailable.as_str().to_string(),
            position_before: None,
            position_after: None,
            position_change: None,
        },
        timeline: vec![],
        context: Context {
            overlapping_acquisitions: 0,
        },
        confidence: None,
        summary: "This backlink isn't tracked for impact. Enable tracking to observe what \
                  happens after acquisition."
            .to_string(),
    }
}

fn event_label(kind: &str) -> &'static str {
    match kind {
        "backlink_found" => "Backlink verified",
        "backlink_lost" => "Backlink lost",
        "backlink_restored" => "Backlink recovered",
        "anchor_changed" => "Anchor changed",
        "target_changed" => "Target changed",
        "dofollow_to_nofollow" => "Changed to nofollow",
        "nofollow_to_dofollow" => "Changed to dofollow",
        _ => "Change observed",
    }
}

// ─── GET impact ───────────────────────────────────────────────────────────────
pub async fn get_impact(
    State(state): State<AppState>,
    Path((org_id, backlink_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
) -> Result<Json<ImpactView>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    let b = backlinks::get(&state.pool, &scope, backlink_id).await?; // 404 if not ours

    let elig = eligibility(&b);
    let existing = impact::get(&state.pool, &scope, backlink_id).await?;
    if elig.is_none() && existing.is_none() {
        return Ok(Json(not_eligible_view()));
    }

    // Ensure an analysis row (lazy create for eligible links; manual opt-in uses POST).
    let analysis = match existing {
        Some(a) => a,
        None => {
            let (t0, src) = resolve_t0(&b);
            impact::create(
                &state.pool,
                &scope,
                Uuid::now_v7(),
                backlink_id,
                b.project_id,
                &b.target_url_normalized,
                t0,
                src,
                elig,
            )
            .await?
        }
    };
    let t0 = analysis.t0;
    let now = Utc::now();
    let page = &analysis.target_url_normalized;

    // Is Search Console connected for this project (property mapped)?
    let gsc_connected = gsc::get(&state.pool, &scope, b.project_id)
        .await?
        .map(|c| c.site_url.is_some())
        .unwrap_or(false);

    // Build the visibility windows + capture the 28d aggregates for ranking.
    let mut vis_windows = Vec::new();
    let mut vis_signals = Vec::new();
    let mut ranking = RankingLayer {
        status: SignalState::Unavailable.as_str().to_string(),
        position_before: None,
        position_after: None,
        position_change: None,
    };
    let mut best_conf: Option<backlio_core::Confidence> = None;
    let mut any_data = false;

    if gsc_connected {
        for &win in WINDOWS {
            let t0d = t0.date_naive();
            let post_end = t0d + Duration::days(win);
            if now.date_naive() < post_end {
                vis_windows.push(VisibilityWindow {
                    label: format!("{win}d"),
                    status: SignalState::Pending.as_str().to_string(),
                    available_on: Some(post_end.to_string()),
                    impressions_before: 0,
                    impressions_after: 0,
                    clicks_before: 0,
                    clicks_after: 0,
                    change_percent: None,
                });
                continue;
            }
            let pre = gsc_daily::page_window(
                &state.pool,
                b.organization_id,
                b.project_id,
                page,
                t0d - Duration::days(win),
                t0d,
            )
            .await
            .unwrap_or_default();
            let post = gsc_daily::page_window(
                &state.pool,
                b.organization_id,
                b.project_id,
                page,
                t0d,
                post_end,
            )
            .await
            .unwrap_or_default();
            if pre.days > 0 || post.days > 0 {
                any_data = true;
            }
            let change = change_percent(
                per_day(pre.impressions, win),
                per_day(post.impressions, win),
            );
            let signal = up_signal(
                change,
                pre.impressions,
                post.impressions,
                RATE_THRESHOLD_PCT,
            );
            vis_signals.push(signal);
            vis_windows.push(VisibilityWindow {
                label: format!("{win}d"),
                status: signal.as_str().to_string(),
                available_on: None,
                impressions_before: pre.impressions,
                impressions_after: post.impressions,
                clicks_before: pre.clicks,
                clicks_after: post.clicks,
                change_percent: change,
            });

            // The 28d window drives ranking + confidence.
            if win == 28 {
                let pb = avg_pos(&pre);
                let pa = avg_pos(&post);
                ranking = RankingLayer {
                    status: position_signal(pb, pa, POSITION_THRESHOLD)
                        .as_str()
                        .to_string(),
                    position_before: pb,
                    position_after: pa,
                    position_change: match (pb, pa) {
                        (Some(a), Some(c)) => Some(c - a),
                        _ => None,
                    },
                };
                best_conf = Some(confidence(post.days, win, post.impressions));
            }
        }
    }

    // Overall status.
    let status = if !gsc_connected {
        SignalState::Unavailable
    } else if vis_windows.iter().all(|w| w.status == "pending") {
        SignalState::Pending
    } else if !any_data {
        SignalState::InsufficientData
    } else {
        SignalState::Positive // placeholder; refined to the headline below
    };

    let vis_status = headline(&vis_signals);
    let visibility = VisibilityLayer {
        status: if gsc_connected && !vis_signals.is_empty() {
            vis_status.as_str().to_string()
        } else if gsc_connected {
            SignalState::Pending.as_str().to_string()
        } else {
            SignalState::Unavailable.as_str().to_string()
        },
        windows: vis_windows,
    };

    // Attribution collision (§44).
    let overlapping =
        impact::overlapping_count(&state.pool, &scope, b.project_id, page, backlink_id, t0, 28)
            .await
            .unwrap_or(0);

    // Timeline from real events + the T0 marker.
    let mut timeline = vec![TimelineItem {
        kind: "t0".to_string(),
        at: t0.to_rfc3339(),
        label: "Backlink acquired (T0)".to_string(),
    }];
    for e in events::list_for_backlink(&state.pool, &scope, backlink_id, 15)
        .await
        .unwrap_or_default()
    {
        timeline.push(TimelineItem {
            kind: e.event_type.clone(),
            at: e.detected_at.to_rfc3339(),
            label: event_label(&e.event_type).to_string(),
        });
    }

    // Deterministic headline status + summary (never causal).
    let overall = match status {
        SignalState::Positive => headline(&[vis_status, parse_signal(&ranking.status)]),
        other => other,
    };
    let summary = build_summary(overall, &visibility, &ranking, gsc_connected, overlapping);
    let confidence = if matches!(
        overall,
        SignalState::Positive | SignalState::Negative | SignalState::Neutral
    ) {
        best_conf.map(|c| c.as_str().to_string())
    } else {
        None
    };

    Ok(Json(ImpactView {
        status: overall.as_str().to_string(),
        eligible: true,
        impact_started_at: Some(t0.to_rfc3339()),
        t0_source: Some(analysis.t0_source.clone()),
        data_sources: if gsc_connected {
            vec!["google_search_console"]
        } else {
            vec![]
        },
        crawl: unavailable_layer("Crawl observation isn't available yet (coming soon)."),
        indexation: unavailable_layer("Indexation data isn't available yet (coming soon)."),
        visibility,
        ranking,
        timeline,
        context: Context {
            overlapping_acquisitions: overlapping,
        },
        confidence,
        summary,
    }))
}

fn parse_signal(s: &str) -> SignalState {
    match s {
        "positive" => SignalState::Positive,
        "negative" => SignalState::Negative,
        "neutral" => SignalState::Neutral,
        "pending" => SignalState::Pending,
        "insufficient_data" => SignalState::InsufficientData,
        _ => SignalState::Unavailable,
    }
}

fn build_summary(
    overall: SignalState,
    vis: &VisibilityLayer,
    ranking: &RankingLayer,
    gsc_connected: bool,
    overlapping: i64,
) -> String {
    if !gsc_connected {
        return "Connect Google Search Console to measure visibility and ranking response."
            .to_string();
    }
    let mut parts: Vec<String> = Vec::new();
    if let Some(w) = vis.windows.iter().find(|w| w.label == "28d") {
        match (w.status.as_str(), w.change_percent) {
            ("pending", _) => parts.push(format!(
                "28-day window completes on {}.",
                w.available_on.clone().unwrap_or_default()
            )),
            (_, Some(c)) => parts.push(format!(
                "Search impressions {} {:.0}% over the 28-day comparison.",
                if c >= 0.0 { "up" } else { "down" },
                c.abs()
            )),
            (_, None) => {}
        }
    }
    if let (Some(a), Some(b)) = (ranking.position_before, ranking.position_after) {
        if (a - b).abs() >= POSITION_THRESHOLD {
            parts.push(format!("Average position moved from {a:.1} to {b:.1}."));
        }
    }
    let head = match overall {
        SignalState::Positive => "Positive post-acquisition signals.",
        SignalState::Negative => "Negative post-acquisition signals.",
        SignalState::Neutral => "No clear change observed.",
        SignalState::Pending => "Collecting data.",
        _ => "No measurable response detected yet.",
    };
    let mut s = head.to_string();
    if !parts.is_empty() {
        s.push(' ');
        s.push_str(&parts.join(" "));
    }
    if overlapping > 0 {
        s.push_str(&format!(
            " Note: {overlapping} other backlink(s) were acquired for this page in the window — \
             observed changes can't be isolated to one link."
        ));
    }
    s
}

// ─── POST enable (manual opt-in) ──────────────────────────────────────────────
pub async fn enable_impact(
    State(state): State<AppState>,
    Path((org_id, backlink_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    let b = backlinks::get(&state.pool, &scope, backlink_id).await?;
    if impact::get(&state.pool, &scope, backlink_id)
        .await?
        .is_some()
    {
        return Ok(StatusCode::NO_CONTENT);
    }
    let (t0, src) = resolve_t0(&b);
    let elig = eligibility(&b).unwrap_or("manual");
    impact::create(
        &state.pool,
        &scope,
        Uuid::now_v7(),
        backlink_id,
        b.project_id,
        &b.target_url_normalized,
        t0,
        src,
        Some(elig),
    )
    .await?;
    Ok(StatusCode::CREATED)
}
