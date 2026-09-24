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

#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub user: UserView,
    pub organizations: Vec<OrgMembershipView>,
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
    }))
}
