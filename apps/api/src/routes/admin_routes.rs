//! Platform admin backoffice (migration 0022). The one sanctioned cross-tenant
//! surface: it lets the SaaS operator see every organization and user and manage
//! platform-admin grants. Every handler goes through [`require_admin`] first,
//! which resolves the session and refuses anyone whose `is_platform_admin` is
//! false — returning `NotFound`, so the surface is invisible to non-admins.

use crate::error::ApiError;
use crate::session::{check_csrf, current};
use crate::state::AppState;
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum_extra::extract::cookie::CookieJar;
use db::entities::User;
use db::repositories::{admin, audit, users};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

/// Resolve the current user and require the platform-admin flag. A non-admin
/// (or anonymous) caller gets `NotFound`, never `Forbidden`, so the existence of
/// the backoffice is not leaked.
async fn require_admin(state: &AppState, jar: &CookieJar) -> Result<User, ApiError> {
    let (user, _session) = current(state, jar).await?;
    if !user.is_platform_admin {
        return Err(ApiError::NotFound);
    }
    Ok(user)
}

#[derive(Serialize)]
pub struct StatsView {
    pub users: i64,
    pub organizations: i64,
    pub projects: i64,
    pub backlinks: i64,
    pub backlinks_live: i64,
    pub backlinks_lost: i64,
}

/// GET /v1/admin/stats — platform-wide totals.
pub async fn stats(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Json<StatsView>, ApiError> {
    require_admin(&state, &jar).await?;
    let s = admin::platform_stats(&state.pool).await?;
    Ok(Json(StatsView {
        users: s.users,
        organizations: s.organizations,
        projects: s.projects,
        backlinks: s.backlinks,
        backlinks_live: s.backlinks_live,
        backlinks_lost: s.backlinks_lost,
    }))
}

#[derive(Serialize)]
pub struct AdminOrgView {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub created_at: String,
    pub suspended: bool,
    pub plan: String,
    pub members: i64,
    pub projects: i64,
    pub backlinks: i64,
    pub owner_email: Option<String>,
}

/// GET /v1/admin/organizations — every workspace on the instance.
pub async fn list_organizations(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Json<Vec<AdminOrgView>>, ApiError> {
    require_admin(&state, &jar).await?;
    let rows = admin::list_organizations(&state.pool).await?;
    Ok(Json(
        rows.into_iter()
            .map(|o| AdminOrgView {
                id: o.id,
                name: o.name,
                slug: o.slug,
                created_at: o.created_at.to_rfc3339(),
                suspended: o.suspended,
                plan: o.plan,
                members: o.members,
                projects: o.projects,
                backlinks: o.backlinks,
                owner_email: o.owner_email,
            })
            .collect(),
    ))
}

#[derive(Deserialize)]
pub struct SuspendBody {
    pub suspended: bool,
}

/// POST /v1/admin/organizations/{org_id}/suspend — freeze/unfreeze a workspace.
pub async fn set_suspended(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<SuspendBody>,
) -> Result<axum::http::StatusCode, ApiError> {
    let admin_user = require_admin(&state, &jar).await?;
    let (_, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let name = admin::set_suspended(&state.pool, org_id, body.suspended).await?;
    let action = if body.suspended {
        "admin.org.suspend"
    } else {
        "admin.org.unsuspend"
    };
    let _ = audit::log(
        &state.pool,
        Some(org_id),
        Some(admin_user.id),
        Some(admin_user.email.as_str()),
        action,
        Some("organization"),
        Some(org_id),
        None,
        Some(json!({ "name": name, "suspended": body.suspended })),
    )
    .await;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct SetPlanBody {
    pub plan: String,
}

/// POST /v1/admin/organizations/{org_id}/plan — override the org's plan (manual,
/// no Stripe). Validates the slug against the plan catalog.
pub async fn set_plan(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<SetPlanBody>,
) -> Result<axum::http::StatusCode, ApiError> {
    let admin_user = require_admin(&state, &jar).await?;
    let (_, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let plan = billing::Plan::from_slug(&body.plan)
        .ok_or_else(|| ApiError::Validation(format!("unknown plan '{}'", body.plan)))?;
    admin::set_plan(&state.pool, org_id, plan.spec().slug).await?;
    let _ = audit::log(
        &state.pool,
        Some(org_id),
        Some(admin_user.id),
        Some(admin_user.email.as_str()),
        "admin.org.plan_change",
        Some("organization"),
        Some(org_id),
        None,
        Some(json!({ "plan": plan.spec().slug })),
    )
    .await;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// DELETE /v1/admin/organizations/{org_id} — permanently delete a workspace and
/// all its data (FK cascades). Irreversible.
pub async fn delete_organization(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
    headers: HeaderMap,
) -> Result<axum::http::StatusCode, ApiError> {
    let admin_user = require_admin(&state, &jar).await?;
    let (_, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let name = admin::delete_organization(&state.pool, org_id).await?;
    // org_id is None here: the org is gone, so an org-scoped audit row would be
    // cascade-deleted with it. Keep the id in resource_id instead.
    let _ = audit::log(
        &state.pool,
        None,
        Some(admin_user.id),
        Some(admin_user.email.as_str()),
        "admin.org.delete",
        Some("organization"),
        Some(org_id),
        Some(json!({ "name": name })),
        None,
    )
    .await;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[derive(Serialize)]
pub struct AuditView {
    pub id: Uuid,
    pub created_at: String,
    pub organization_id: Option<Uuid>,
    pub org_name: Option<String>,
    pub actor_email: Option<String>,
    pub action: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<Uuid>,
    pub after: Option<String>,
}

/// GET /v1/admin/audit — recent audit entries across all tenants.
pub async fn list_audit(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Json<Vec<AuditView>>, ApiError> {
    require_admin(&state, &jar).await?;
    let rows = audit::list_recent(&state.pool, 200).await?;
    Ok(Json(
        rows.into_iter()
            .map(|a| AuditView {
                id: a.id,
                created_at: a.created_at.to_rfc3339(),
                organization_id: a.organization_id,
                org_name: a.org_name,
                actor_email: a.actor_email,
                action: a.action,
                resource_type: a.resource_type,
                resource_id: a.resource_id,
                after: a.after,
            })
            .collect(),
    ))
}

#[derive(Serialize)]
pub struct AdminUserView {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub is_platform_admin: bool,
    pub email_verified: bool,
    pub created_at: String,
    pub organizations: i64,
}

/// GET /v1/admin/users — every account on the instance.
pub async fn list_users(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Json<Vec<AdminUserView>>, ApiError> {
    require_admin(&state, &jar).await?;
    let rows = admin::list_users(&state.pool).await?;
    Ok(Json(
        rows.into_iter()
            .map(|u| AdminUserView {
                id: u.id,
                email: u.email,
                name: u.name,
                is_platform_admin: u.is_platform_admin,
                email_verified: u.email_verified_at.is_some(),
                created_at: u.created_at.to_rfc3339(),
                organizations: u.organizations,
            })
            .collect(),
    ))
}

#[derive(Deserialize)]
pub struct SetAdminBody {
    pub is_admin: bool,
}

/// POST /v1/admin/users/{user_id}/admin — grant or revoke platform admin.
/// Guards against self-demotion so the operator can't lock themselves out.
pub async fn set_admin(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<SetAdminBody>,
) -> Result<Json<AdminUserView>, ApiError> {
    let (me, session) = current(&state, &jar).await?;
    if !me.is_platform_admin {
        return Err(ApiError::NotFound);
    }
    check_csrf(&jar, &headers, &session)?;
    if me.id == user_id && !body.is_admin {
        return Err(ApiError::Validation(
            "you cannot revoke your own platform-admin access".into(),
        ));
    }
    let u = users::set_platform_admin(&state.pool, user_id, body.is_admin).await?;
    Ok(Json(AdminUserView {
        id: u.id,
        email: u.email,
        name: u.name,
        is_platform_admin: u.is_platform_admin,
        email_verified: u.email_verified_at.is_some(),
        created_at: u.created_at.to_rfc3339(),
        organizations: 0,
    }))
}

// ─── Founder-ops read endpoints (docs/19) ────────────────────────────────────
// Real data only, from tables that already exist. The docs/19 items are grouped
// into three endpoints (ops / billing / usage) to keep the console to a few
// round-trips.

#[derive(Serialize)]
pub struct QueueView {
    pub pending: i64,
    pub processing: i64,
    pub failed: i64,
    pub dead: i64,
    pub oldest_pending_at: Option<String>,
    pub last_completed_at: Option<String>,
    pub created_last_hour: i64,
}

#[derive(Serialize)]
pub struct MonitoringView {
    pub checks: i64,
    pub failed: i64,
    pub avg_duration_ms: Option<i64>,
    pub lost_detected: i64,
}

#[derive(Serialize)]
pub struct EmailView {
    pub sent: i64,
    pub failed: i64,
}

#[derive(Serialize)]
pub struct OpsView {
    pub window_secs: i64,
    pub queue: QueueView,
    pub monitoring: MonitoringView,
    pub email: EmailView,
}

#[derive(Deserialize)]
pub struct OpsQuery {
    pub window_hours: Option<i64>,
}

/// GET /v1/admin/ops — queue, monitoring (over `?window_hours`, default 24) and
/// alert-email health. All derived from existing tables (docs/19 §quick-wins).
pub async fn ops(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(q): Query<OpsQuery>,
) -> Result<Json<OpsView>, ApiError> {
    require_admin(&state, &jar).await?;
    let hours = q.window_hours.unwrap_or(24).clamp(1, 24 * 30);
    let secs = hours * 3600;
    let h = admin::ops_health(&state.pool, secs as f64).await?;
    Ok(Json(OpsView {
        window_secs: secs,
        queue: QueueView {
            pending: h.q_pending,
            processing: h.q_processing,
            failed: h.q_failed,
            dead: h.q_dead,
            oldest_pending_at: h.q_oldest_pending_at.map(|t| t.to_rfc3339()),
            last_completed_at: h.q_last_completed_at.map(|t| t.to_rfc3339()),
            created_last_hour: h.q_created_1h,
        },
        monitoring: MonitoringView {
            checks: h.checks,
            failed: h.checks_failed,
            avg_duration_ms: h.avg_duration_ms.map(|v| v.round() as i64),
            lost_detected: h.lost_detected,
        },
        email: EmailView {
            sent: h.email_sent,
            failed: h.email_failed,
        },
    }))
}

#[derive(Serialize)]
pub struct PlanAmount {
    pub plan: String,
    pub subscribers: i64,
    pub mrr_cents: i64,
}

#[derive(Serialize)]
pub struct BillingView {
    pub mrr_cents: i64,
    pub paying: i64,
    pub trialing: i64,
    /// Trial funnel: live trials vs. trials that elapsed without converting.
    pub trialing_active: i64,
    pub trialing_expired: i64,
    /// Of resolved trials (converted + expired), the % that converted. `null`
    /// when no trial has resolved yet.
    pub conversion_pct: Option<i64>,
    pub past_due: i64,
    pub canceled: i64,
    pub comped: i64,
    pub by_plan: Vec<PlanAmount>,
}

/// GET /v1/admin/billing — MRR, the trial funnel, status counts and a by-plan
/// breakdown. Admin-comped subscriptions are excluded from money. Prices come
/// from the plan catalog (docs/19).
pub async fn billing(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Json<BillingView>, ApiError> {
    require_admin(&state, &jar).await?;
    let counts = admin::billing_counts(&state.pool).await?;
    let per_plan = admin::paying_by_plan(&state.pool).await?;
    let mut by_plan = Vec::new();
    let mut mrr_cents = 0i64;
    let mut paying = 0i64;
    for pc in per_plan {
        let price = billing::Plan::from_slug(&pc.plan)
            .map(|p| p.spec().price_cents as i64)
            .unwrap_or(0);
        let line = price * pc.n;
        mrr_cents += line;
        paying += pc.n;
        by_plan.push(PlanAmount {
            plan: pc.plan,
            subscribers: pc.n,
            mrr_cents: line,
        });
    }
    // Conversion = converted / (converted + lapsed trials). "Converted" counts
    // paying subscriptions; comped ones aren't conversions.
    let resolved = paying + counts.trialing_expired;
    let conversion_pct = (resolved > 0).then(|| paying * 100 / resolved);
    Ok(Json(BillingView {
        mrr_cents,
        paying,
        trialing: counts.trialing,
        trialing_active: counts.trialing_active,
        trialing_expired: counts.trialing_expired,
        conversion_pct,
        past_due: counts.past_due,
        canceled: counts.canceled,
        comped: counts.comped,
        by_plan,
    }))
}

#[derive(Serialize)]
pub struct SubscriptionView {
    pub id: Uuid,
    pub name: String,
    pub owner_email: Option<String>,
    pub plan: String,
    pub status: String,
    /// Per-customer MRR from the catalog (0 for trials, comped or non-paying).
    pub mrr_cents: i64,
    pub comped: bool,
    pub trial_ends_at: Option<String>,
    pub current_period_end: Option<String>,
    pub cancel_at_period_end: bool,
    pub canceled_at: Option<String>,
}

/// GET /v1/admin/subscriptions — one row per org that has a subscription, with
/// its billing state and per-customer MRR (docs/20 — P7). Renewal, trial and
/// cancel dates are the ones the Stripe webhook now writes (P4).
pub async fn list_subscriptions(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Json<Vec<SubscriptionView>>, ApiError> {
    require_admin(&state, &jar).await?;
    let rows = admin::list_subscriptions(&state.pool).await?;
    let out = rows
        .into_iter()
        .map(|s| {
            let paying = matches!(s.status.as_str(), "active" | "past_due") && !s.comped;
            let mrr_cents = if paying {
                billing::Plan::from_slug(&s.plan)
                    .map(|p| p.spec().price_cents as i64)
                    .unwrap_or(0)
            } else {
                0
            };
            SubscriptionView {
                id: s.id,
                name: s.name,
                owner_email: s.owner_email,
                plan: s.plan,
                status: s.status,
                mrr_cents,
                comped: s.comped,
                trial_ends_at: s.trial_end.map(|t| t.to_rfc3339()),
                current_period_end: s.current_period_end.map(|t| t.to_rfc3339()),
                cancel_at_period_end: s.cancel_at_period_end,
                canceled_at: s.canceled_at.map(|t| t.to_rfc3339()),
            }
        })
        .collect();
    Ok(Json(out))
}

#[derive(Serialize)]
pub struct UsageView {
    pub id: Uuid,
    pub name: String,
    pub plan: String,
    pub backlinks: i64,
    pub limit: Option<i64>,
    pub pct: Option<i64>,
}

/// GET /v1/admin/usage/limits — workspaces by monitored-backlink usage vs their
/// plan cap, busiest first (docs/19). Limits come from the plan catalog.
pub async fn usage_limits(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Json<Vec<UsageView>>, ApiError> {
    require_admin(&state, &jar).await?;
    let rows = admin::usage_vs_limit(&state.pool).await?;
    let out = rows
        .into_iter()
        .map(|r| {
            let limit = billing::Plan::from_slug(&r.plan)
                .and_then(|p| p.spec().backlink_limit)
                .map(|l| l as i64);
            let pct = limit.map(|l| {
                if l > 0 {
                    (r.backlinks * 100 / l).min(999)
                } else {
                    0
                }
            });
            UsageView {
                id: r.id,
                name: r.name,
                plan: r.plan,
                backlinks: r.backlinks,
                limit,
                pct,
            }
        })
        .collect();
    Ok(Json(out))
}

#[derive(Serialize)]
pub struct SourceRow {
    pub source: String,
    pub signups: i64,
}

#[derive(Serialize)]
pub struct AnalyticsView {
    pub window_secs: i64,
    pub signups: i64,
    pub trials_started: i64,
    pub activated: i64,
    pub subscribed: i64,
    pub by_source: Vec<SourceRow>,
}

/// GET /v1/admin/analytics — the signup→trial→activation→paid funnel and a
/// signup-by-source attribution breakdown over `?window_hours` (default 30 days,
/// docs/20 §P8). All from the self-hosted analytics_events sink.
pub async fn analytics(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(q): Query<OpsQuery>,
) -> Result<Json<AnalyticsView>, ApiError> {
    require_admin(&state, &jar).await?;
    let hours = q.window_hours.unwrap_or(24 * 30).clamp(1, 24 * 365);
    let secs = hours * 3600;
    let f = db::repositories::analytics::funnel(&state.pool, secs as f64).await?;
    let src = db::repositories::analytics::signups_by_source(&state.pool, secs as f64).await?;
    Ok(Json(AnalyticsView {
        window_secs: secs,
        signups: f.signups,
        trials_started: f.trials_started,
        activated: f.activated,
        subscribed: f.subscribed,
        by_source: src
            .into_iter()
            .map(|s| SourceRow {
                source: s.source,
                signups: s.signups,
            })
            .collect(),
    }))
}

#[derive(Serialize)]
pub struct WaitlistRow {
    pub email: String,
    pub created_at: String,
    pub source: Option<String>,
}

#[derive(Serialize)]
pub struct WaitlistView {
    pub total: i64,
    /// Signups that have not received a confirmation email yet.
    pub pending: i64,
    pub recent: Vec<WaitlistRow>,
}

/// GET /v1/admin/waitlist — total, pending-confirmation count + recent signups.
pub async fn waitlist(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Json<WaitlistView>, ApiError> {
    require_admin(&state, &jar).await?;
    let total = db::repositories::waitlist::count(&state.pool).await?;
    let pending = db::repositories::waitlist::pending_count(&state.pool).await?;
    let recent = db::repositories::waitlist::recent(&state.pool, 100).await?;
    Ok(Json(WaitlistView {
        total,
        pending,
        recent: recent
            .into_iter()
            .map(|r| WaitlistRow {
                email: r.email,
                created_at: r.created_at.to_rfc3339(),
                source: r.utm_source,
            })
            .collect(),
    }))
}

#[derive(Serialize)]
pub struct ResendResult {
    pub sent: i64,
    pub failed: i64,
    pub remaining: i64,
}

/// POST /v1/admin/waitlist/resend — send the confirmation email to every signup
/// that hasn't received one yet (idempotent: a successful send is stamped, a
/// failure stays pending for a later retry). Admin only, CSRF-checked.
pub async fn resend_waitlist_confirmations(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
) -> Result<Json<ResendResult>, ApiError> {
    let admin_user = require_admin(&state, &jar).await?;
    let (_, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;

    let pending = db::repositories::waitlist::pending_confirmations(&state.pool, 500).await?;
    let (mut sent, mut failed) = (0i64, 0i64);
    for email in &pending {
        match notifications::waitlist::send_confirmation(email).await {
            Ok(()) => {
                let _ = db::repositories::waitlist::mark_confirmed(&state.pool, email).await;
                sent += 1;
            }
            Err(e) => {
                failed += 1;
                tracing::warn!(error = %e, "waitlist confirmation resend failed");
            }
        }
    }
    let _ = audit::log(
        &state.pool,
        None,
        Some(admin_user.id),
        Some(admin_user.email.as_str()),
        "admin.waitlist.resend_confirmations",
        Some("waitlist"),
        None,
        None,
        Some(json!({ "sent": sent, "failed": failed })),
    )
    .await;
    let remaining = db::repositories::waitlist::pending_count(&state.pool).await?;
    Ok(Json(ResendResult {
        sent,
        failed,
        remaining,
    }))
}
