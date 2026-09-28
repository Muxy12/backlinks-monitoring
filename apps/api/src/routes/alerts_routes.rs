//! Alert destinations (§37, migration 0024): org-level webhook / Slack URLs that
//! new notifications are POSTed to. Managing them requires Member; the outbound
//! POST (here for "test", and in the tick for real delivery) is SSRF-guarded.

use crate::error::ApiError;
use crate::session::{check_csrf, current};
use crate::state::AppState;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum_extra::extract::cookie::CookieJar;
use common::Role;
use crawler::CrawlerConfig;
use db::authorize_org;
use db::repositories::alerts;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize)]
pub struct DestinationView {
    pub id: Uuid,
    pub kind: String,
    pub target: String,
    pub name: String,
    pub enabled: bool,
    pub digest: String,
    pub last_status: Option<String>,
    pub last_error: Option<String>,
    pub last_delivered_at: Option<String>,
    pub created_at: String,
}

impl From<db::entities::AlertDestination> for DestinationView {
    fn from(d: db::entities::AlertDestination) -> Self {
        DestinationView {
            id: d.id,
            kind: d.kind,
            target: d.target,
            name: d.name,
            enabled: d.enabled,
            digest: d.digest,
            last_status: d.last_status,
            last_error: d.last_error,
            last_delivered_at: d.last_delivered_at.map(|t| t.to_rfc3339()),
            created_at: d.created_at.to_rfc3339(),
        }
    }
}

pub async fn list_destinations(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
) -> Result<Json<Vec<DestinationView>>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Viewer)?;
    let rows = alerts::list(&state.pool, &scope).await?;
    Ok(Json(rows.into_iter().map(DestinationView::from).collect()))
}

#[derive(Deserialize)]
pub struct CreateDestinationBody {
    pub kind: String,
    pub target: String,
    pub name: Option<String>,
    /// immediate | hourly | daily; defaults to immediate.
    pub digest: Option<String>,
}

const DIGESTS: &[&str] = &["immediate", "hourly", "daily"];

/// Light validation — kind is webhook|slack, target is an http(s) URL with a
/// host. The full SSRF check runs when we actually POST (test / delivery), so
/// this only rejects obviously wrong input (no url-crate dependency needed).
fn validate(kind: &str, target: &str) -> Result<(), ApiError> {
    let t = target.trim();
    match kind {
        "email" => {
            let ok = t.find('@').is_some_and(|i| i > 0 && i < t.len() - 1)
                && !t.contains(char::is_whitespace);
            if !ok {
                return Err(ApiError::Validation(
                    "target must be an email address".into(),
                ));
            }
            Ok(())
        }
        "webhook" | "slack" => {
            let rest = t
                .strip_prefix("https://")
                .or_else(|| t.strip_prefix("http://"))
                .ok_or_else(|| ApiError::Validation("target must be an http(s) URL".into()))?;
            let host = rest.split(['/', '?', '#']).next().unwrap_or("");
            let host = host.rsplit('@').next().unwrap_or(host); // strip userinfo
            let host = host.split(':').next().unwrap_or(host); // strip port
            if host.is_empty() {
                return Err(ApiError::Validation("target must have a host".into()));
            }
            Ok(())
        }
        _ => Err(ApiError::Validation(
            "kind must be 'webhook', 'slack' or 'email'".into(),
        )),
    }
}

pub async fn create_destination(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<CreateDestinationBody>,
) -> Result<(axum::http::StatusCode, Json<DestinationView>), ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;

    let kind = body.kind.trim();
    let target = body.target.trim();
    validate(kind, target)?;
    let name = body.name.as_deref().unwrap_or("").trim();
    if name.len() > 120 {
        return Err(ApiError::Validation("name is too long".into()));
    }
    let digest = body.digest.as_deref().unwrap_or("immediate");
    if !DIGESTS.contains(&digest) {
        return Err(ApiError::Validation(
            "digest must be 'immediate', 'hourly' or 'daily'".into(),
        ));
    }
    let dest = alerts::create(
        &state.pool,
        &scope,
        Uuid::now_v7(),
        kind,
        target,
        name,
        digest,
    )
    .await?;
    Ok((axum::http::StatusCode::CREATED, Json(dest.into())))
}

#[derive(Deserialize)]
pub struct UpdateDestinationBody {
    pub enabled: Option<bool>,
    /// immediate | hourly | daily
    pub digest: Option<String>,
}

/// PATCH a destination — toggle enabled and/or change the digest cadence.
pub async fn update_destination(
    State(state): State<AppState>,
    Path((org_id, dest_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<UpdateDestinationBody>,
) -> Result<Json<DestinationView>, ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    if body.enabled.is_none() && body.digest.is_none() {
        return Err(ApiError::Validation("nothing to update".into()));
    }
    if let Some(e) = body.enabled {
        alerts::set_enabled(&state.pool, &scope, dest_id, e).await?;
    }
    if let Some(d) = body.digest.as_deref() {
        if !DIGESTS.contains(&d) {
            return Err(ApiError::Validation(
                "digest must be 'immediate', 'hourly' or 'daily'".into(),
            ));
        }
        alerts::set_digest(&state.pool, &scope, dest_id, d).await?;
    }
    let dest = alerts::get(&state.pool, &scope, dest_id).await?;
    Ok(Json(dest.into()))
}

pub async fn delete_destination(
    State(state): State<AppState>,
    Path((org_id, dest_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
) -> Result<axum::http::StatusCode, ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    alerts::delete(&state.pool, &scope, dest_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[derive(Serialize)]
pub struct TestResult {
    pub ok: bool,
    pub http_status: Option<u16>,
    pub detail: String,
}

/// POST a test payload to the destination now, so the user gets immediate
/// feedback that their URL works. SSRF-guarded like real delivery.
pub async fn test_destination(
    State(state): State<AppState>,
    Path((org_id, dest_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
) -> Result<Json<TestResult>, ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    let dest = alerts::get(&state.pool, &scope, dest_id).await?;
    let cfg = CrawlerConfig {
        allow_private_addresses: state.cfg.allow_private_fetch,
        ..CrawlerConfig::default()
    };
    match worker::deliver::send_test(&state.pool, &cfg, &dest).await {
        Ok(code) => Ok(Json(TestResult {
            ok: true,
            http_status: Some(code),
            detail: format!("Delivered (HTTP {code})."),
        })),
        Err(e) => Ok(Json(TestResult {
            ok: false,
            http_status: None,
            detail: e,
        })),
    }
}
