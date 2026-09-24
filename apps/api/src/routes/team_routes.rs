//! Team management (Phase 1): invite members, list them, accept an invitation.
//! Managing invitations requires Admin+; the members list is visible to any
//! member. Accepting is done by the invited user themselves. Invitations never
//! grant ownership (schema CHECK), and the raw token is returned once on create
//! so the app can build an invite link (email delivery lands later).

use crate::error::ApiError;
use crate::session::{check_csrf, current};
use crate::state::AppState;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum_extra::extract::cookie::CookieJar;
use common::Role;
use db::authorize_org;
use db::repositories::{invitations, members};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

const INVITE_TTL_DAYS: i64 = 7;

#[derive(Debug, Serialize)]
pub struct MemberView {
    pub user_id: Uuid,
    pub email: String,
    pub name: String,
    pub role: String,
}

pub async fn list_members(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
) -> Result<Json<Vec<MemberView>>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    let rows = members::list(&state.pool, &scope).await?;
    Ok(Json(
        rows.into_iter()
            .map(|m| MemberView {
                user_id: m.user_id,
                email: m.email,
                name: m.name,
                role: m.role,
            })
            .collect(),
    ))
}

#[derive(Debug, Deserialize)]
pub struct CreateInvitationBody {
    pub email: String,
    /// admin | member | viewer (never owner).
    pub role: String,
}

#[derive(Debug, Serialize)]
pub struct InvitationView {
    pub id: Uuid,
    pub email: String,
    pub role: String,
    pub expires_at: String,
    /// The raw acceptance token — shown once so the app can build an invite
    /// link. `None` on list (only the create response carries it).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

pub async fn create_invitation(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<CreateInvitationBody>,
) -> Result<(StatusCode, Json<InvitationView>), ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Admin)?; // only admins/owners may invite

    let email = body.email.trim();
    if email.is_empty() || email.len() > 320 || !email.contains('@') {
        return Err(ApiError::Validation("a valid email is required".into()));
    }
    let role = Role::from_str(body.role.trim().to_ascii_lowercase().as_str())
        .map_err(|_| ApiError::Validation("role must be admin, member or viewer".into()))?;
    if role == Role::Owner {
        return Err(ApiError::Validation(
            "an invitation cannot grant ownership".into(),
        ));
    }

    let token = auth::token::Token::generate();
    let expires_at = chrono::Utc::now() + chrono::Duration::days(INVITE_TTL_DAYS);
    let inv = invitations::create(
        &state.pool,
        &scope,
        Uuid::now_v7(),
        email,
        role.as_str(),
        user.id,
        &token.hash,
        expires_at,
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(InvitationView {
            id: inv.id,
            email: inv.email,
            role: inv.role,
            expires_at: inv.expires_at.to_rfc3339(),
            token: Some(token.secret),
        }),
    ))
}

pub async fn list_invitations(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
) -> Result<Json<Vec<InvitationView>>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Admin)?;
    let rows = invitations::list_pending(&state.pool, &scope).await?;
    Ok(Json(
        rows.into_iter()
            .map(|i| InvitationView {
                id: i.id,
                email: i.email,
                role: i.role,
                expires_at: i.expires_at.to_rfc3339(),
                token: None,
            })
            .collect(),
    ))
}

pub async fn revoke_invitation(
    State(state): State<AppState>,
    Path((org_id, invitation_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Admin)?;
    invitations::revoke(&state.pool, &scope, invitation_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct AcceptInvitationBody {
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct AcceptedView {
    pub organization_id: Uuid,
    pub role: String,
}

/// POST /v1/invitations/accept — the invited, logged-in user joins the org. The
/// token is matched by hash; the invitation's email must match the caller's, so
/// a leaked token cannot add an unintended account.
pub async fn accept_invitation(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<AcceptInvitationBody>,
) -> Result<Json<AcceptedView>, ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;

    let hash = auth::token::hash_token(body.token.trim());
    let now = chrono::Utc::now();
    let inv = invitations::find_live_by_token_hash(&state.pool, &hash, now)
        .await
        .map_err(|_| ApiError::NotFound)?;

    if !user.email.eq_ignore_ascii_case(&inv.email) {
        // Do not reveal which org/email — treat as not found for this caller.
        return Err(ApiError::NotFound);
    }

    invitations::accept(&state.pool, inv.id, inv.organization_id, user.id, &inv.role).await?;
    Ok(Json(AcceptedView {
        organization_id: inv.organization_id,
        role: inv.role,
    }))
}
