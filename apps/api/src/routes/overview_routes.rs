//! The Overview / command center aggregation (dashboard redesign). ONE endpoint
//! composes every section the Overview needs — summary KPIs, the prioritized
//! "Needs Your Attention" queue + next best action, backlink growth, campaign
//! performance, top opportunities, competitive position, and recent activity —
//! so the page makes a single request. It reads ONLY the database (no DataForSEO,
//! crawler, or LLM): opening the dashboard must stay cheap (§36/§37/§61).
//!
//! Money is always per-currency (there is no FX in the system); current-state
//! KPIs ignore the date range, which only affects growth + period deltas (§34).
//! Campaign/opportunity sections are null when the campaigns BETA flag is off.

use crate::error::ApiError;
use crate::routes::campaigns_routes::StageCount;
use crate::routes::ops_routes::CurrencyAmount;
use crate::session::current;
use crate::state::AppState;
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum_extra::extract::cookie::CookieJar;
use backlio_core::{AttentionKind, attention_priority, attention_sort_key};
use db::authorize_org;
use db::repositories::{backlinks, dashboard, overview, prospects};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Response shape ───────────────────────────────────────────────────────────
#[derive(Serialize)]
pub struct OverviewView {
    pub summary: Summary,
    pub attention: Vec<AttentionItemView>,
    pub next_best_action: Option<AttentionItemView>,
    pub backlink_growth: BacklinkGrowth,
    pub campaign_performance: Option<CampaignPerformanceView>,
    pub top_opportunities: Vec<OpportunityRow>,
    pub competitive_position: Option<CompetitivePosition>,
    pub recent_activity: Vec<ActivityItem>,
    pub generated_at: String,
}

#[derive(Serialize)]
pub struct Summary {
    pub link_health: LinkHealth,
    pub protected_value: Vec<CurrencyAmount>,
    pub active_campaigns: ActiveCampaigns,
    pub value_at_risk: Vec<CurrencyAmount>,
}

#[derive(Serialize)]
pub struct LinkHealth {
    /// null when nothing is monitored/eligible yet.
    pub percentage: Option<f64>,
    pub healthy: i64,
    pub eligible: i64,
}

#[derive(Serialize)]
pub struct ActiveCampaigns {
    pub active: i64,
    pub draft: i64,
    pub total: i64,
}

#[derive(Serialize, Clone)]
pub struct AttentionItemView {
    pub priority: String,
    pub kind: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub value_at_risk: Option<CurrencyAmount>,
    pub action: String,
    pub route: String,
}

#[derive(Serialize)]
pub struct GrowthPoint {
    pub day: String,
    pub referring_domains: i64,
    pub protected: i64,
    pub live: i64,
}

#[derive(Serialize)]
pub struct BacklinkGrowth {
    pub series: Vec<GrowthPoint>,
    pub new_count: i64,
    pub lost_count: i64,
    pub net_count: i64,
}

#[derive(Serialize)]
pub struct CampaignPerformanceView {
    pub active: i64,
    pub budget: Vec<CurrencyAmount>,
    pub spend: Vec<CurrencyAmount>,
    pub prospects_total: i64,
    pub stage_counts: Vec<StageCount>,
}

#[derive(Serialize)]
pub struct OpportunityRow {
    pub prospect_id: Uuid,
    pub campaign_id: Uuid,
    pub domain: String,
    pub score: Option<i32>,
    pub score_reason: Option<String>,
    pub estimated_cost: Option<String>,
    pub currency: Option<String>,
}

#[derive(Serialize)]
pub struct CompetitivePosition {
    pub our_referring_domains: i64,
    pub competitor_referring_domains: i64,
    pub biggest_gap_domain: Option<String>,
    pub biggest_gap_competitors: Option<i64>,
}

#[derive(Serialize)]
pub struct ActivityItem {
    pub event_type: String,
    pub source_domain: String,
    pub target_url: String,
    pub backlink_id: Uuid,
    pub detected_at: String,
}

#[derive(Deserialize)]
pub struct OverviewQuery {
    pub range: Option<String>,
}

fn amounts(rows: Vec<(String, String)>) -> Vec<CurrencyAmount> {
    rows.into_iter()
        .map(|(currency, amount)| CurrencyAmount { currency, amount })
        .collect()
}

/// Range slug → lookback days. Default 30 days.
fn range_days(range: Option<&str>) -> i64 {
    match range {
        Some("3m") => 90,
        Some("6m") => 180,
        Some("1y") => 365,
        _ => 30,
    }
}

/// Cost text ("180.00") → minor units (cents) for ranking; 0 when unknown.
fn money_units(cost: &Option<String>) -> i64 {
    cost.as_deref()
        .and_then(|c| c.parse::<f64>().ok())
        .map(|f| (f * 100.0).round() as i64)
        .unwrap_or(0)
}

/// One gathered item plus its descending sort key.
struct Ranked {
    key: (u8, i64, i64),
    item: AttentionItemView,
}

pub async fn overview(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
    Query(q): Query<OverviewQuery>,
) -> Result<(StatusCode, Json<OverviewView>), ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    let beta = state.cfg.campaigns_beta;

    let days = range_days(q.range.as_deref());
    let since_dt = chrono::Utc::now() - chrono::Duration::days(days);
    let since_date = since_dt.date_naive();

    // ── Summary KPIs (current-state; ignore range) ──
    let (healthy, eligible) = overview::link_health(&state.pool, &scope).await?;
    let link_health = LinkHealth {
        percentage: (eligible > 0)
            .then(|| ((healthy as f64 / eligible as f64) * 1000.0).round() / 10.0),
        healthy,
        eligible,
    };
    let protected_value = amounts(dashboard::value_protected(&state.pool, &scope).await?);
    let value_at_risk_money = dashboard::value_at_risk(&state.pool, &scope).await?;
    let value_at_risk = amounts(value_at_risk_money);
    let camp_status = overview::campaign_status_counts(&state.pool, &scope).await?;
    let mut active = 0i64;
    let mut draft = 0i64;
    let mut total = 0i64;
    for (st, n) in &camp_status {
        total += n;
        match st.as_str() {
            "active" => active += n,
            "draft" => draft += n,
            _ => {}
        }
    }

    // ── Backlink growth ──
    let series: Vec<GrowthPoint> = overview::growth_series(&state.pool, &scope, since_date)
        .await?
        .into_iter()
        .map(|r| GrowthPoint {
            day: r.day.to_string(),
            referring_domains: r.referring_domains,
            protected: r.protected,
            live: r.live,
        })
        .collect();
    let (new_count, lost_count) = overview::growth_counts(&state.pool, &scope, since_dt).await?;
    let backlink_growth = BacklinkGrowth {
        series,
        new_count,
        lost_count,
        net_count: new_count - lost_count,
    };

    // ── Competitive position (reduced) ──
    let (our_rd, comp_rd) = overview::referring_domain_counts(&state.pool, &scope).await?;
    let competitive_position = if comp_rd > 0 {
        let gap = overview::top_gap(&state.pool, &scope).await?;
        Some(CompetitivePosition {
            our_referring_domains: our_rd,
            competitor_referring_domains: comp_rd,
            biggest_gap_domain: gap.as_ref().map(|(d, _)| d.clone()),
            biggest_gap_competitors: gap.map(|(_, c)| c),
        })
    } else {
        None
    };

    // ── Recent activity ──
    let recent_activity: Vec<ActivityItem> = overview::recent_events(&state.pool, &scope, 8)
        .await?
        .into_iter()
        .map(|e| ActivityItem {
            event_type: e.event_type,
            source_domain: e.source_domain,
            target_url: e.target_url,
            backlink_id: e.backlink_id,
            detected_at: e.detected_at.to_rfc3339(),
        })
        .collect();

    // ── Needs Your Attention (gather → prioritize → sort → top 5) ──
    let mut ranked: Vec<Ranked> = Vec::new();

    for r in overview::at_risk_items(&state.pool, &scope, 10).await? {
        let lost = matches!(r.status.as_str(), "lost" | "suspected_lost");
        let kind = if lost {
            AttentionKind::BacklinkLost
        } else {
            AttentionKind::BacklinkChanged
        };
        let units = money_units(&r.cost);
        let has_value = units > 0;
        let p = attention_priority(kind, has_value);
        let value_at_risk = match (&r.cost, &r.currency) {
            (Some(amount), Some(currency)) => Some(CurrencyAmount {
                currency: currency.clone(),
                amount: amount.clone(),
            }),
            _ => None,
        };
        ranked.push(Ranked {
            key: attention_sort_key(p, units, 0),
            item: AttentionItemView {
                priority: p.as_str().to_string(),
                kind: kind.as_str().to_string(),
                title: if lost {
                    "Backlink lost".into()
                } else {
                    "Backlink needs attention".into()
                },
                subtitle: Some(format!("{} → {}", r.source_domain, r.target_url)),
                value_at_risk,
                action: if lost {
                    "Recover".into()
                } else {
                    "Review".into()
                },
                route: if lost {
                    "/backlinks?filter=lost".into()
                } else {
                    "/backlinks".into()
                },
            },
        });
    }

    // Warranty expiring within the lookahead window.
    let cutoff = chrono::Utc::now().date_naive() + chrono::Duration::days(14);
    for b in backlinks::warranty_expiring_for_org(&state.pool, &scope, cutoff, 5).await? {
        let units = money_units(&b.cost);
        let p = attention_priority(AttentionKind::WarrantyExpiring, units > 0);
        let value_at_risk = match (&b.cost, &b.currency) {
            (Some(amount), Some(currency)) => Some(CurrencyAmount {
                currency: currency.clone(),
                amount: amount.clone(),
            }),
            _ => None,
        };
        ranked.push(Ranked {
            key: attention_sort_key(p, units, 0),
            item: AttentionItemView {
                priority: p.as_str().to_string(),
                kind: AttentionKind::WarrantyExpiring.as_str().to_string(),
                title: "Guarantee expiring soon".into(),
                subtitle: Some(b.source_domain.clone()),
                value_at_risk,
                action: "View".into(),
                route: "/backlinks".into(),
            },
        });
    }

    // Campaign-side attention (only when the BETA is on).
    if beta {
        let stage_counts = overview::prospect_stage_counts_org(&state.pool, &scope).await?;
        let awaiting: i64 = stage_counts
            .iter()
            .filter(|(s, _)| matches!(s.as_str(), "contacted" | "negotiating"))
            .map(|(_, n)| n)
            .sum();
        if awaiting > 0 {
            let p = attention_priority(AttentionKind::ProspectsAwaiting, false);
            ranked.push(Ranked {
                key: attention_sort_key(p, 0, 0),
                item: AttentionItemView {
                    priority: p.as_str().to_string(),
                    kind: AttentionKind::ProspectsAwaiting.as_str().to_string(),
                    title: format!(
                        "{awaiting} prospect{} awaiting a decision",
                        if awaiting == 1 { "" } else { "s" }
                    ),
                    subtitle: None,
                    value_at_risk: None,
                    action: "Review".into(),
                    route: "/campaigns".into(),
                },
            });
        }

        // Budget nearly exhausted: any active-campaign currency where spend >= 90%.
        let budget = overview::active_campaign_budget(&state.pool, &scope).await?;
        let spend = overview::campaign_spend_org(&state.pool, &scope).await?;
        let low = budget.iter().any(|(cur, b)| {
            let b: f64 = b.parse().unwrap_or(0.0);
            let s: f64 = spend
                .iter()
                .find(|(c, _)| c == cur)
                .and_then(|(_, v)| v.parse().ok())
                .unwrap_or(0.0);
            b > 0.0 && s >= b * 0.9
        });
        if low {
            let p = attention_priority(AttentionKind::CampaignBudgetLow, false);
            ranked.push(Ranked {
                key: attention_sort_key(p, 0, 0),
                item: AttentionItemView {
                    priority: p.as_str().to_string(),
                    kind: AttentionKind::CampaignBudgetLow.as_str().to_string(),
                    title: "Campaign budget nearly exhausted".into(),
                    subtitle: None,
                    value_at_risk: None,
                    action: "View".into(),
                    route: "/campaigns".into(),
                },
            });
        }
    }

    // Sort descending by (priority, value, recency); keep the top 5.
    ranked.sort_by_key(|r| std::cmp::Reverse(r.key));
    let attention: Vec<AttentionItemView> = ranked.into_iter().map(|r| r.item).take(5).collect();
    let next_best_action = attention.first().cloned();

    // ── Campaign performance + top opportunities (BETA only) ──
    let (campaign_performance, top_opportunities) = if beta {
        let stage_rows = overview::prospect_stage_counts_org(&state.pool, &scope).await?;
        let prospects_total: i64 = stage_rows.iter().map(|(_, n)| n).sum();
        let perf = if total > 0 {
            Some(CampaignPerformanceView {
                active,
                budget: amounts(overview::active_campaign_budget(&state.pool, &scope).await?),
                spend: amounts(overview::campaign_spend_org(&state.pool, &scope).await?),
                prospects_total,
                stage_counts: stage_rows
                    .into_iter()
                    .map(|(stage, count)| StageCount { stage, count })
                    .collect(),
            })
        } else {
            None
        };
        let opps: Vec<OpportunityRow> = prospects::top_opportunities(&state.pool, &scope, 5)
            .await?
            .into_iter()
            .map(|p| OpportunityRow {
                prospect_id: p.id,
                campaign_id: p.campaign_id,
                domain: p.domain,
                score: p.score,
                score_reason: p.score_reason,
                estimated_cost: p.estimated_cost,
                currency: p.currency,
            })
            .collect();
        (perf, opps)
    } else {
        (None, Vec::new())
    };

    Ok((
        StatusCode::OK,
        Json(OverviewView {
            summary: Summary {
                link_health,
                protected_value,
                active_campaigns: ActiveCampaigns {
                    active,
                    draft,
                    total,
                },
                value_at_risk,
            },
            attention,
            next_best_action,
            backlink_growth,
            campaign_performance,
            top_opportunities,
            competitive_position,
            recent_activity,
            generated_at: chrono::Utc::now().to_rfc3339(),
        }),
    ))
}
