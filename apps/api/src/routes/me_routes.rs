//! `GET /v1/me` — the current user and their organization memberships
//! (docs/09 §Auth).

use crate::error::ApiError;
use crate::routes::auth_routes::UserView;
use crate::session::{check_csrf, clear_cookies, current};
use crate::state::AppState;
use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum_extra::extract::cookie::CookieJar;
use db::repositories::{orgs, users};
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Deserialize)]
pub struct DeleteAccountBody {
    /// The user re-types their email to confirm this irreversible action.
    pub confirm_email: String,
}

/// DELETE /v1/me — permanently delete the current user's account (GDPR erasure).
/// Session-authenticated + CSRF-protected, and guarded by an email re-type. Any
/// organization the user solely owns is deleted with all its data; the session
/// cookies are cleared on success.
pub async fn delete_account(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<DeleteAccountBody>,
) -> Result<impl IntoResponse, ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    if !body.confirm_email.trim().eq_ignore_ascii_case(&user.email) {
        return Err(ApiError::Validation(
            "type your account email to confirm deletion".into(),
        ));
    }

    // Cancel any live Stripe subscriptions on the orgs this user solely owns, so
    // deleting the account also stops billing. Best-effort and BEFORE the DB
    // deletion (which drops the Stripe ids): a Stripe hiccup must not block the
    // user's right to erasure.
    if let Some(secret) = billing::stripe::secret_key() {
        match users::sole_owned_subscription_ids(&state.pool, user.id).await {
            Ok(subs) => {
                for sub in subs {
                    if let Err(e) = billing::stripe::cancel_subscription(&secret, &sub).await {
                        tracing::warn!(error = %e, "stripe subscription not canceled on account deletion");
                    }
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "could not list subscriptions before account deletion")
            }
        }
    }

    users::delete_account(&state.pool, user.id).await?;
    Ok((clear_cookies(jar), StatusCode::NO_CONTENT))
}
