//! Tenant-scoped resource routes: clients (§20) and projects (§21), nested
//! under `/v1/organizations/{org_id}/...` (docs/09 §1.1). Every handler resolves
//! `OrgScope` from the path org id via `authorize_org` — the `org_id` is an
//! assertion to be checked against membership, never a trusted input (§18). A
//! non-member gets 404 (ADR-0004); a member lacking the write role gets 403.

use crate::error::ApiError;
use crate::session::{check_csrf, current};
use crate::state::AppState;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum_extra::extract::cookie::CookieJar;
use common::Role;
use db::authorize_org;
use db::repositories::{clients, projects};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Clients ─────────────────────────────────────────────────────────────────
#[derive(Debug, Serialize)]
pub struct ClientView {
    pub id: Uuid,
    pub name: String,
    pub company_name: Option<String>,
    pub website: Option<String>,
}

impl From<db::entities::Client> for ClientView {
    fn from(c: db::entities::Client) -> Self {
        ClientView {
            id: c.id,
            name: c.name,
            company_name: c.company_name,
            website: c.website,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateClientBody {
    pub name: String,
}

pub async fn create_client(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<CreateClientBody>,
) -> Result<(StatusCode, Json<ClientView>), ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?; // members and up may create (docs/05 §2)

    let name = body.name.trim();
    if name.is_empty() || name.len() > 200 {
        return Err(ApiError::Validation("name must be 1–200 characters".into()));
    }
    let client = clients::create(&state.pool, &scope, Uuid::now_v7(), name).await?;
    Ok((StatusCode::CREATED, Json(client.into())))
}

pub async fn list_clients(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
) -> Result<Json<Vec<ClientView>>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    let rows = clients::list(&state.pool, &scope).await?;
    Ok(Json(rows.into_iter().map(ClientView::from).collect()))
}

// ─── Projects ────────────────────────────────────────────────────────────────
#[derive(Debug, Serialize)]
pub struct ProjectView {
    pub id: Uuid,
    pub name: String,
    pub domain: String,
    pub check_frequency_secs: i64,
    pub status: String,
}

impl From<db::entities::Project> for ProjectView {
    fn from(p: db::entities::Project) -> Self {
        ProjectView {
            id: p.id,
            name: p.name,
            domain: p.domain,
            check_frequency_secs: p.check_frequency_secs,
            status: p.status,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateProjectBody {
    pub name: String,
    pub domain: String,
    /// Optional; defaults to 7 days. Bounded to the schema's 1h–180d range.
    pub check_frequency_hours: Option<i64>,
}

pub async fn create_project(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<CreateProjectBody>,
) -> Result<(StatusCode, Json<ProjectView>), ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;

    let name = body.name.trim();
    let domain = body.domain.trim();
    if name.is_empty() || name.len() > 200 {
        return Err(ApiError::Validation("name must be 1–200 characters".into()));
    }
    if domain.is_empty() {
        return Err(ApiError::Validation("domain is required".into()));
    }
    let hours = body.check_frequency_hours.unwrap_or(24 * 7);
    if !(1..=24 * 180).contains(&hours) {
        return Err(ApiError::Validation(
            "check frequency must be 1 hour – 180 days".into(),
        ));
    }
    let project = projects::create(
        &state.pool,
        &scope,
        Uuid::now_v7(),
        name,
        domain,
        hours * 3600,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(project.into())))
}

pub async fn list_projects(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
) -> Result<Json<Vec<ProjectView>>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    let rows = projects::list(&state.pool, &scope).await?;
    Ok(Json(rows.into_iter().map(ProjectView::from).collect()))
}

pub async fn get_project(
    State(state): State<AppState>,
    Path((org_id, project_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
) -> Result<Json<ProjectView>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    let project = projects::get(&state.pool, &scope, project_id).await?;
    Ok(Json(project.into()))
}
