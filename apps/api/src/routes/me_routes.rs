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
            // Post-Link Impact / GSC is open to everyone: the Google OAuth app is
            // verified for public use (sensitive scope `webmasters.readonly`),
            // so the flag now gates only on the integration being configured.
            // The whole Impact UI keys off this flag.
            gsc: crate::routes::gsc_routes::configured(),
        },
    }))
}
