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
use db::repositories::{admin, users};
use serde::{Deserialize, Serialize};
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
                members: o.members,
                projects: o.projects,
                backlinks: o.backlinks,
                owner_email: o.owner_email,
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
