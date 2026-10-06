//! `GET /v1/me` — the current user and their organization memberships
//! (docs/09 §Auth).

use crate::error::ApiError;
use crate::routes::auth_routes::UserView;
use crate::session::current;
use crate::state::AppState;
use axum::Json;
use axum::extract::State;
use axum_extra::extract::cookie::CookieJar;
use db::repositories::orgs;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct OrgMembershipView {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub role: String,
}

/// Runtime feature flags surfaced to the UI so it can hide BETA/unconfigured
/// surfaces (same idea as billing's `configured`). Add a field per flag.
#[derive(Debug, Serialize)]
pub struct Features {
    pub campaigns_beta: bool,
    /// Google Search Console integration is configured (Post-Link Impact).
    pub gsc: bool,
}

#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub user: UserView,
    pub organizations: Vec<OrgMembershipView>,
    pub features: Features,
}

pub async fn me(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Json<MeResponse>, ApiError> {
    let (user, _session) = current(&state, &jar).await?;
    let orgs = orgs::list_for_user(&state.pool, user.id).await?;
    let organizations = orgs
        .into_iter()
        .map(|(o, role)| OrgMembershipView {
            id: o.id,
            name: o.name,
            slug: o.slug,
            role: role.to_string(),
        })
        .collect();
    Ok(Json(MeResponse {
        user: user.into(),
        organizations,
        features: Features {
            campaigns_beta: state.cfg.campaigns_beta,
            // Post-Link Impact / GSC is admin-only until the Google OAuth app is
            // verified for public use (a sensitive scope). Public users see
            // "coming soon"; the whole Impact UI gates on this flag. Drop the
            // `&& is_platform_admin` to open it to everyone after verification.
            gsc: crate::routes::gsc_routes::configured() && user.is_platform_admin,
        },
    }))
}
