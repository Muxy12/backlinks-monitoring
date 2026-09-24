//! Notification preferences (§37): a member controls, per org, which backlink
//! event types notify them and on which channel. A missing row means the
//! default (enabled, immediate). Preferences are the caller's own.

use crate::error::ApiError;
use crate::session::{check_csrf, current};
use crate::state::AppState;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum_extra::extract::cookie::CookieJar;
use db::authorize_org;
use db::repositories::notification_prefs;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The backlink change events a user can be notified about (the `event_type`
/// enum, docs/03).
const EVENT_TYPES: &[&str] = &[
    "backlink_found",
    "backlink_lost",
    "backlink_restored",
    "anchor_changed",
    "target_changed",
    "dofollow_to_nofollow",
    "nofollow_to_dofollow",
    "rel_changed",
    "source_redirected",
    "source_404",
    "source_error",
    "target_redirected",
    "target_404",
    "target_error",
    "noindex_detected",
    "canonical_changed",
];
const CHANNELS: &[&str] = &["in_app", "email", "slack", "webhook", "discord"];
const DIGESTS: &[&str] = &["immediate", "hourly", "daily"];

#[derive(Debug, Serialize)]
pub struct PreferenceView {
    pub event_type: String,
    pub channel: String,
    pub enabled: bool,
    pub digest: String,
}

impl From<db::entities::NotificationPreference> for PreferenceView {
    fn from(p: db::entities::NotificationPreference) -> Self {
        PreferenceView {
            event_type: p.event_type,
            channel: p.channel,
            enabled: p.enabled,
            digest: p.digest,
        }
    }
}

pub async fn list_preferences(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
) -> Result<Json<Vec<PreferenceView>>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    let rows = notification_prefs::list(&state.pool, user.id, scope.organization_id()).await?;
    Ok(Json(rows.into_iter().map(PreferenceView::from).collect()))
}

#[derive(Debug, Deserialize)]
pub struct PreferenceInput {
    pub event_type: String,
    pub channel: String,
    pub enabled: bool,
    /// immediate | hourly | daily; defaults to immediate.
    pub digest: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SetPreferencesBody {
    pub preferences: Vec<PreferenceInput>,
}

pub async fn set_preferences(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<SetPreferencesBody>,
) -> Result<Json<Vec<PreferenceView>>, ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;

    if body.preferences.len() > 200 {
        return Err(ApiError::Validation(
            "too many preferences in one call".into(),
        ));
    }
    // Validate everything before writing anything.
    for p in &body.preferences {
        if !EVENT_TYPES.contains(&p.event_type.as_str()) {
            return Err(ApiError::Validation(format!(
                "unknown event_type {:?}",
                p.event_type
            )));
        }
        if !CHANNELS.contains(&p.channel.as_str()) {
            return Err(ApiError::Validation(format!(
                "unknown channel {:?}",
                p.channel
            )));
        }
        if let Some(d) = p.digest.as_deref()
            && !DIGESTS.contains(&d)
        {
            return Err(ApiError::Validation(format!("unknown digest {d:?}")));
        }
    }
    for p in &body.preferences {
        let digest = p.digest.as_deref().unwrap_or("immediate");
        notification_prefs::upsert(
            &state.pool,
            user.id,
            scope.organization_id(),
            &p.event_type,
            &p.channel,
            p.enabled,
            digest,
        )
        .await?;
    }

    let rows = notification_prefs::list(&state.pool, user.id, scope.organization_id()).await?;
    Ok(Json(rows.into_iter().map(PreferenceView::from).collect()))
}
