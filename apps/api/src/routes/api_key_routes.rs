//! Subscriber API keys (docs/product/22 §1) — create / list / revoke the
//! `lp_live_…` Bearer keys that authenticate the MCP server and the keyed REST
//! API. Admin-only (a key can act for the whole workspace). The secret is
//! returned exactly once, on creation; only its hash is stored.

use crate::error::ApiError;
use crate::session::{check_csrf, current};
use crate::state::AppState;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum_extra::extract::cookie::CookieJar;
use common::Role;
use db::authorize_org;
use db::repositories::api_keys;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The scopes a key may be granted (docs/product/22 §1). A key can only ever
/// narrow what the user can already do; these map to the MCP tools' scopes.
const VALID_SCOPES: &[&str] = &[
    "read:portfolio",
    "read:discovery",
    "write:discovery",
    "read:geo",
    "read:marketplace",
];
/// Granted when the caller doesn't specify scopes: everything read-only.
const DEFAULT_SCOPES: &[&str] = &[
    "read:portfolio",
    "read:discovery",
    "read:geo",
    "read:marketplace",
];

#[derive(Serialize)]
pub struct ApiKeyView {
    pub id: Uuid,
    pub name: String,
    pub prefix: String,
    pub scopes: Vec<String>,
    pub last_used_at: Option<String>,
    pub created_at: String,
    pub revoked_at: Option<String>,
}

impl From<db::entities::ApiKeyRow> for ApiKeyView {
    fn from(k: db::entities::ApiKeyRow) -> Self {
        ApiKeyView {
            id: k.id,
            name: k.name,
            prefix: k.prefix,
            scopes: k.scopes,
            last_used_at: k.last_used_at.map(|t| t.to_rfc3339()),
            created_at: k.created_at.to_rfc3339(),
            revoked_at: k.revoked_at.map(|t| t.to_rfc3339()),
        }
    }
}

#[derive(Serialize)]
pub struct ApiKeysResponse {
    pub keys: Vec<ApiKeyView>,
}

/// `GET /organizations/{org_id}/api-keys` — list the workspace's keys.
pub async fn list_keys(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
) -> Result<Json<ApiKeysResponse>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Admin)?;

    let keys = api_keys::list(&state.pool, &scope).await?;
    Ok(Json(ApiKeysResponse {
        keys: keys.into_iter().map(ApiKeyView::from).collect(),
    }))
}

#[derive(Deserialize)]
pub struct CreateKeyBody {
    pub name: String,
    pub scopes: Option<Vec<String>>,
}

/// The create response — the only time the full secret is ever returned.
#[derive(Serialize)]
pub struct CreatedKey {
    pub id: Uuid,
    pub name: String,
    pub prefix: String,
    pub scopes: Vec<String>,
    /// The full key (`lp_live_…`). Shown once — store it now; it cannot be
    /// retrieved again.
    pub secret: String,
}

/// `POST /organizations/{org_id}/api-keys` — mint a key. Admin only.
pub async fn create_key(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
    headers: axum::http::HeaderMap,
    Json(body): Json<CreateKeyBody>,
) -> Result<(StatusCode, Json<CreatedKey>), ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Admin)?;

    let name = body.name.trim();
    if name.is_empty() {
        return Err(ApiError::Validation("name must not be empty".into()));
    }

    // Validate requested scopes against the known set; default to read-only.
    let scopes: Vec<String> = match body.scopes {
        Some(requested) => {
            for s in &requested {
                if !VALID_SCOPES.contains(&s.as_str()) {
                    return Err(ApiError::Validation(format!("unknown scope: {s}")));
                }
            }
            requested
        }
        None => DEFAULT_SCOPES.iter().map(|s| s.to_string()).collect(),
    };

    let key = auth::generate_api_key();
    let id = api_keys::create(
        &state.pool,
        scope.organization_id(),
        name,
        &key.prefix,
        &key.hash,
        &scopes,
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(CreatedKey {
            id,
            name: name.to_string(),
            prefix: key.prefix,
            scopes,
            secret: key.secret,
        }),
    ))
}

/// `DELETE /organizations/{org_id}/api-keys/{key_id}` — revoke a key.
pub async fn revoke_key(
    State(state): State<AppState>,
    Path((org_id, key_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: axum::http::HeaderMap,
) -> Result<StatusCode, ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Admin)?;

    api_keys::revoke(&state.pool, &scope, key_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
