//! Tags (§35): create/list tags and attach/detach them to backlinks. Everything
//! is tenant-scoped via `authorize_org`, and both ids in an attach are validated
//! to belong to the org, so a cross-tenant tag or backlink yields 404.

use crate::error::ApiError;
use crate::session::{check_csrf, current};
use crate::state::AppState;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum_extra::extract::cookie::CookieJar;
use common::Role;
use db::authorize_org;
use db::repositories::{backlinks, tags};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct TagView {
    pub id: Uuid,
    pub name: String,
    pub color: Option<String>,
}

impl From<db::entities::Tag> for TagView {
    fn from(t: db::entities::Tag) -> Self {
        TagView {
            id: t.id,
            name: t.name,
            color: t.color,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateTagBody {
    pub name: String,
    /// Optional hex color `#RRGGBB`.
    pub color: Option<String>,
}

fn valid_color(c: &str) -> bool {
    let b = c.as_bytes();
    b.len() == 7 && b[0] == b'#' && c[1..].bytes().all(|x| x.is_ascii_hexdigit())
}

pub async fn create_tag(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<CreateTagBody>,
) -> Result<(StatusCode, Json<TagView>), ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;

    let name = body.name.trim();
    if name.is_empty() || name.len() > 100 {
        return Err(ApiError::Validation("name must be 1–100 characters".into()));
    }
    let color = body
        .color
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    if let Some(c) = color
        && !valid_color(c)
    {
        return Err(ApiError::Validation("color must be #RRGGBB".into()));
    }
    let tag = tags::create(&state.pool, &scope, Uuid::now_v7(), name, color).await?;
    Ok((StatusCode::CREATED, Json(tag.into())))
}

pub async fn list_tags(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
) -> Result<Json<Vec<TagView>>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    let rows = tags::list(&state.pool, &scope).await?;
    Ok(Json(rows.into_iter().map(TagView::from).collect()))
}

pub async fn list_backlink_tags(
    State(state): State<AppState>,
    Path((org_id, backlink_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
) -> Result<Json<Vec<TagView>>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    // 404 if the backlink is not in this org, before listing its tags.
    backlinks::get(&state.pool, &scope, backlink_id).await?;
    let rows = tags::for_backlink(&state.pool, &scope, backlink_id).await?;
    Ok(Json(rows.into_iter().map(TagView::from).collect()))
}

#[derive(Debug, Deserialize)]
pub struct AttachTagBody {
    pub tag_id: Uuid,
}

pub async fn attach_tag(
    State(state): State<AppState>,
    Path((org_id, backlink_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<AttachTagBody>,
) -> Result<StatusCode, ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    // Both must belong to this org (404 otherwise) — never rely on an FK error.
    backlinks::get(&state.pool, &scope, backlink_id).await?;
    tags::get(&state.pool, &scope, body.tag_id).await?;
    tags::attach(&state.pool, &scope, backlink_id, body.tag_id).await?;
    // 204 whether newly attached or already present (idempotent).
    Ok(StatusCode::NO_CONTENT)
}

pub async fn detach_tag(
    State(state): State<AppState>,
    Path((org_id, backlink_id, tag_id)): Path<(Uuid, Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    tags::detach(&state.pool, &scope, backlink_id, tag_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
