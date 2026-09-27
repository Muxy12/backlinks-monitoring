//! Platform admin backoffice (migration 0022). The one sanctioned cross-tenant
//! surface: it lets the SaaS operator see every organization and user and manage
//! platform-admin grants. Every handler goes through [`require_admin`] first,
//! which resolves the session and refuses anyone whose `is_platform_admin` is
//! false — returning `NotFound`, so the surface is invisible to non-admins.

use crate::error::ApiError;
use crate::session::{check_csrf, current};
use crate::state::AppState;
use axum::Json;
use axum::extract::{Path, State};
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
