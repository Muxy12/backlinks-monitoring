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

/// Enforce the org's plan backlink cap before adding `count` (§39). Existing
/// links are never touched; only new adds over the cap are blocked.
async fn enforce_backlink_quota(
    state: &AppState,
    scope: &db::OrgScope,
    count: u32,
) -> Result<(), ApiError> {
    let plan =
        db::repositories::billing::plan_for_org(&state.pool, scope.organization_id()).await?;
    let current = db::repositories::backlinks::count_active(&state.pool, scope).await? as u32;
    match billing::Entitlements::new(plan).can_add_backlinks(current, count) {
        billing::AddOutcome::Allowed { .. } => Ok(()),
        billing::AddOutcome::OverLimit { limit, .. } => Err(ApiError::PlanLimit(format!(
            "plan limit of {limit} monitored backlinks reached — upgrade to add more"
        ))),
    }
}

// ─── Clients ─────────────────────────────────────────────────────────────────
#[derive(Debug, Serialize)]
pub struct ClientView {
    pub id: Uuid,
    pub name: String,
    pub company_name: Option<String>,
    pub website: Option<String>,
    pub contact_name: Option<String>,
    pub contact_email: Option<String>,
}

impl From<db::entities::Client> for ClientView {
    fn from(c: db::entities::Client) -> Self {
        ClientView {
            id: c.id,
            name: c.name,
            company_name: c.company_name,
            website: c.website,
            contact_name: c.contact_name,
            contact_email: c.contact_email,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateClientBody {
    pub name: String,
    pub company_name: Option<String>,
    pub website: Option<String>,
    pub contact_name: Option<String>,
    pub contact_email: Option<String>,
    pub notes: Option<String>,
}

fn clean(s: Option<&str>) -> Option<&str> {
    s.map(str::trim).filter(|v| !v.is_empty())
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
    let email = clean(body.contact_email.as_deref());
    if let Some(e) = email
        && !e.contains('@')
    {
        return Err(ApiError::Validation(
            "contact_email must be an email".into(),
        ));
    }
    let extra = clients::NewClient {
        company_name: clean(body.company_name.as_deref()),
        website: clean(body.website.as_deref()),
        contact_name: clean(body.contact_name.as_deref()),
        contact_email: email,
        notes: clean(body.notes.as_deref()),
    };
    let client = clients::create(&state.pool, &scope, Uuid::now_v7(), name, extra).await?;
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

pub async fn delete_client(
    State(state): State<AppState>,
    Path((org_id, client_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    clients::delete(&state.pool, &scope, client_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ─── Projects ────────────────────────────────────────────────────────────────
#[derive(Debug, Serialize)]
pub struct ProjectView {
    pub id: Uuid,
    pub name: String,
    pub domain: String,
    pub check_frequency_secs: i64,
    pub status: String,
    pub client_id: Option<Uuid>,
}

impl From<db::entities::Project> for ProjectView {
    fn from(p: db::entities::Project) -> Self {
        ProjectView {
            id: p.id,
            name: p.name,
            domain: p.domain,
            check_frequency_secs: p.check_frequency_secs,
            status: p.status,
            client_id: p.client_id,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateProjectBody {
    pub name: String,
    pub domain: String,
    /// Optional; defaults to 7 days. Bounded to the schema's 1h–180d range.
    pub check_frequency_hours: Option<i64>,
    /// Optional client to file this project under (§20); validated in-org.
    pub client_id: Option<Uuid>,
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
    if let Some(cid) = body.client_id {
        clients::get(&state.pool, &scope, cid).await?; // 404 if not in org
    }
    let project = projects::create(
        &state.pool,
        &scope,
        Uuid::now_v7(),
        name,
        domain,
        hours * 3600,
        body.client_id,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(project.into())))
}

#[derive(Debug, Deserialize)]
pub struct UpdateProjectBody {
    pub name: Option<String>,
    pub check_frequency_hours: Option<i64>,
    /// Reassign (or clear via explicit null) the client (§20).
    #[serde(default, deserialize_with = "de_double_opt")]
    pub client_id: Option<Option<Uuid>>,
}

pub async fn update_project(
    State(state): State<AppState>,
    Path((org_id, project_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<UpdateProjectBody>,
) -> Result<Json<ProjectView>, ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;

    let name = match body.name.as_deref().map(str::trim) {
        Some(n) if n.is_empty() || n.len() > 200 => {
            return Err(ApiError::Validation("name must be 1–200 characters".into()));
        }
        other => other,
    };
    let secs = match body.check_frequency_hours {
        Some(h) if !(1..=24 * 180).contains(&h) => {
            return Err(ApiError::Validation(
                "check frequency must be 1 hour – 180 days".into(),
            ));
        }
        Some(h) => Some(h * 3600),
        None => None,
    };
    if name.is_none() && secs.is_none() && body.client_id.is_none() {
        return Err(ApiError::Validation("nothing to update".into()));
    }
    // A newly-assigned client must be in the org (a null clears, no check).
    if let Some(Some(cid)) = body.client_id {
        clients::get(&state.pool, &scope, cid).await?;
    }
    let project =
        projects::update(&state.pool, &scope, project_id, name, secs, body.client_id).await?;
    Ok(Json(project.into()))
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

// ─── Backlinks (§23) ─────────────────────────────────────────────────────────
/// How many days before a warranty lapses we flag it as "expiring soon" in the
/// read view and the daily sweep (§32).
pub const WARRANTY_SOON_DAYS: i64 = 14;

#[derive(Debug, Serialize)]
pub struct BacklinkView {
    pub id: Uuid,
    pub source_url: String,
    pub target_url: String,
    pub source_domain: String,
    pub target_domain: String,
    pub campaign_id: Option<Uuid>,
    pub publisher_id: Option<Uuid>,
    pub expected_anchor_text: Option<String>,
    pub expected_rel_type: String,
    pub status: String,
    pub cost: Option<String>,
    pub currency: Option<String>,
    pub warranty_type: String,
    pub warranty_value: Option<i32>,
    pub warranty_expiration: Option<String>,
    /// Computed at read against today: none/lifetime/active/expiring_soon/expired.
    pub warranty_status: String,
    // ─── What we last actually observed (docs/06, docs/08) ────────────────────
    /// Anchor text found in the link on the source page.
    pub detected_anchor_text: Option<String>,
    /// The exact href found (important for redirects/params).
    pub detected_target_url: Option<String>,
    /// Raw `rel` tokens found on the link.
    pub detected_rel: Option<Vec<String>>,
    pub is_dofollow: Option<bool>,
    pub is_nofollow: Option<bool>,
    pub is_sponsored: Option<bool>,
    pub is_ugc: Option<bool>,
    pub source_http_status: Option<i32>,
    pub target_http_status: Option<i32>,
    /// RFC3339 timestamps.
    pub first_checked_at: Option<String>,
    pub last_seen_at: Option<String>,
    pub last_checked_at: Option<String>,
    /// Link nature observed on the last check: "text" | "image" | null.
    pub link_type: Option<String>,
    pub image_alt: Option<String>,
    /// When the link went `lost` (RFC3339), null if currently present.
    pub lost_at: Option<String>,
    /// Where the target URL lands after following its own redirects.
    pub target_final_url: Option<String>,
}

impl From<db::entities::Backlink> for BacklinkView {
    fn from(b: db::entities::Backlink) -> Self {
        let kind = backlio_core::WarrantyType::from_db(&b.warranty_type);
        let today = chrono::Utc::now().date_naive();
        let warranty_status =
            backlio_core::warranty_status(kind, b.warranty_expiration, today, WARRANTY_SOON_DAYS)
                .as_str()
                .to_string();
        let rel = b.detected_rel_attributes;
        let is_nofollow = rel.as_ref().map(|a| a.iter().any(|r| r == "nofollow"));
        let is_sponsored = rel.as_ref().map(|a| a.iter().any(|r| r == "sponsored"));
        let is_ugc = rel.as_ref().map(|a| a.iter().any(|r| r == "ugc"));
        BacklinkView {
            id: b.id,
            source_url: b.source_url,
            target_url: b.target_url,
            source_domain: b.source_domain,
            target_domain: b.target_domain,
            campaign_id: b.campaign_id,
            publisher_id: b.publisher_id,
            expected_anchor_text: b.expected_anchor_text,
            expected_rel_type: b.expected_rel_type,
            status: b.status,
            cost: b.cost,
            currency: b.currency,
            warranty_type: b.warranty_type,
            warranty_value: b.warranty_value,
            warranty_expiration: b.warranty_expiration.map(|d| d.to_string()),
            warranty_status,
            detected_anchor_text: b.detected_anchor_text,
            detected_target_url: b.detected_target_url,
            is_dofollow: b.detected_is_dofollow,
            is_nofollow,
            is_sponsored,
            is_ugc,
            detected_rel: rel,
            source_http_status: b.source_http_status,
            target_http_status: b.target_http_status,
            first_checked_at: b.first_checked_at.map(|d| d.to_rfc3339()),
            last_seen_at: b.last_seen_at.map(|d| d.to_rfc3339()),
            last_checked_at: b.last_checked_at.map(|d| d.to_rfc3339()),
            link_type: b.link_type,
            image_alt: b.image_alt,
            lost_at: b.lost_at.map(|d| d.to_rfc3339()),
            target_final_url: b.target_final_url,
        }
    }
}

#[derive(serde::Serialize)]
pub struct OccurrenceView {
    pub id: Uuid,
    pub fingerprint: String,
    pub anchor_text: Option<String>,
    pub link_type: Option<String>,
    pub image_alt: Option<String>,
    pub href_raw: Option<String>,
    pub rel: Option<Vec<String>>,
    pub is_nofollow: bool,
    pub is_sponsored: bool,
    pub is_ugc: bool,
    pub placement: String,
    pub surrounding_text: Option<String>,
    pub position: Option<i32>,
    pub first_seen_at: String,
    pub last_seen_at: String,
}

impl From<db::entities::LinkOccurrence> for OccurrenceView {
    fn from(o: db::entities::LinkOccurrence) -> Self {
        OccurrenceView {
            id: o.id,
            fingerprint: o.fingerprint,
            anchor_text: o.anchor_text,
            link_type: o.link_type,
            image_alt: o.image_alt,
            href_raw: o.href_raw,
            rel: o.rel_raw,
            is_nofollow: o.is_nofollow,
            is_sponsored: o.is_sponsored,
            is_ugc: o.is_ugc,
            placement: o.placement,
            surrounding_text: o.surrounding_text,
            position: o.position,
            first_seen_at: o.first_seen_at.to_rfc3339(),
            last_seen_at: o.last_seen_at.to_rfc3339(),
        }
    }
}

/// GET a backlink's live physical occurrences (docs/06 §3.4).
pub async fn list_backlink_occurrences(
    State(state): State<AppState>,
    Path((org_id, backlink_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
) -> Result<Json<Vec<OccurrenceView>>, ApiError> {
    let (user, _session) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Viewer)?;
    let occs = db::repositories::link_occurrences::list(&state.pool, org_id, backlink_id).await?;
    Ok(Json(occs.into_iter().map(OccurrenceView::from).collect()))
}

#[derive(Debug, Deserialize)]
pub struct CreateBacklinkBody {
    pub project_id: Uuid,
    /// Optional campaign/publisher assignment (§22, §31). Each is validated to
    /// belong to this org before use, so a cross-tenant id yields 404.
    pub campaign_id: Option<Uuid>,
    pub publisher_id: Option<Uuid>,
    pub source_url: String,
    pub target_url: String,
    pub expected_anchor_text: Option<String>,
    pub expected_rel_type: Option<String>,
    pub cost: Option<f64>,
    pub currency: Option<String>,
    /// Warranty (§32): "none" (default), "days", "months", or "lifetime".
    /// `days`/`months` require a positive `warranty_value`.
    pub warranty_type: Option<String>,
    pub warranty_value: Option<i32>,
}

fn is_http_url(s: &str) -> bool {
    let s = s.trim();
    s.starts_with("http://") || s.starts_with("https://")
}

pub async fn create_backlink(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<CreateBacklinkBody>,
) -> Result<(StatusCode, Json<BacklinkView>), ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;

    if !is_http_url(&body.source_url) || !is_http_url(&body.target_url) {
        return Err(ApiError::Validation(
            "source_url and target_url must be http(s) URLs".into(),
        ));
    }
    let rel = body.expected_rel_type.as_deref().unwrap_or("dofollow");
    if !matches!(rel, "dofollow" | "nofollow" | "sponsored" | "ugc") {
        return Err(ApiError::Validation(
            "expected_rel_type must be dofollow/nofollow/sponsored/ugc".into(),
        ));
    }
    let cost_str = match (body.cost, body.currency.as_deref()) {
        (Some(_), None) => {
            return Err(ApiError::Validation(
                "currency is required when cost is set".into(),
            ));
        }
        (Some(c), Some(_)) if c < 0.0 => {
            return Err(ApiError::Validation("cost cannot be negative".into()));
        }
        (Some(c), Some(_)) => Some(format!("{c:.2}")),
        (None, _) => None,
    };
    // Warranty (§32): validate the type, require a positive value for
    // days/months, and materialize the expiration date from today so the DB can
    // index it. `none`/`lifetime` never carry a value or a date.
    let warranty_type = body.warranty_type.as_deref().unwrap_or("none");
    let warranty_kind = backlio_core::WarrantyType::from_db(warranty_type);
    if !matches!(warranty_type, "none" | "days" | "months" | "lifetime") {
        return Err(ApiError::Validation(
            "warranty_type must be none/days/months/lifetime".into(),
        ));
    }
    if warranty_kind.needs_value() && !matches!(body.warranty_value, Some(v) if v > 0) {
        return Err(ApiError::Validation(
            "warranty_value must be a positive number of days/months".into(),
        ));
    }
    let warranty_value = if warranty_kind.needs_value() {
        body.warranty_value
    } else {
        None // ignore a stray value on none/lifetime
    };
    let today = chrono::Utc::now().date_naive();
    let warranty_expiration =
        backlio_core::warranty_expiration(warranty_kind, warranty_value, today);

    // The project must belong to this org (returns 404 otherwise).
    db::repositories::projects::get(&state.pool, &scope, body.project_id).await?;
    // Same for an optional campaign/publisher — validate org membership so a
    // cross-tenant id can never be stitched onto a backlink (404, not FK error).
    if let Some(cid) = body.campaign_id {
        db::repositories::campaigns::get(&state.pool, &scope, cid).await?;
    }
    if let Some(pid) = body.publisher_id {
        db::repositories::publishers::get(&state.pool, &scope, pid).await?;
    }

    // Plan limit (§39): block a new backlink over the cap; existing links keep
    // being monitored. Free/new orgs default to Starter.
    enforce_backlink_quota(&state, &scope, 1).await?;

    let anchor = body
        .expected_anchor_text
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let new = db::repositories::backlinks::NewBacklink {
        project_id: body.project_id,
        campaign_id: body.campaign_id,
        publisher_id: body.publisher_id,
        source_url: body.source_url.trim(),
        target_url: body.target_url.trim(),
        expected_anchor_text: anchor,
        expected_rel_type: rel,
        cost: cost_str.as_deref(),
        currency: body.currency.as_deref(),
        warranty_type,
        warranty_value,
        warranty_expiration,
    };
    let backlink = db::repositories::backlinks::create(&state.pool, &scope, Uuid::now_v7(), new)
        .await
        .map_err(|e| match e {
            db::DbError::Config(_) => ApiError::Validation("could not parse a URL".into()),
            other => other.into(),
        })?;
    Ok((StatusCode::CREATED, Json(backlink.into())))
}

// ─── Edit a backlink (§36) ────────────────────────────────────────────────────
/// serde helper: distinguish an absent field (`None`) from one explicitly set to
/// a value or `null` (`Some(_)`), so PATCH can tell "keep" from "clear".
fn de_double_opt<'de, T, D>(de: D) -> Result<Option<Option<T>>, D::Error>
where
    T: serde::Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    serde::Deserialize::deserialize(de).map(Some)
}

#[derive(Debug, Deserialize)]
pub struct UpdateBacklinkBody {
    #[serde(default, deserialize_with = "de_double_opt")]
    pub expected_anchor_text: Option<Option<String>>,
    /// Set only (the column is NOT NULL); omit to keep.
    pub expected_rel_type: Option<String>,
    #[serde(default, deserialize_with = "de_double_opt")]
    pub cost: Option<Option<f64>>,
    #[serde(default, deserialize_with = "de_double_opt")]
    pub currency: Option<Option<String>>,
    #[serde(default, deserialize_with = "de_double_opt")]
    pub campaign_id: Option<Option<Uuid>>,
    #[serde(default, deserialize_with = "de_double_opt")]
    pub publisher_id: Option<Option<Uuid>>,
}

/// PATCH …/backlinks/{id} — edit mutable fields (§36). Source/target URLs are
/// immutable (they define the link). A field present in the body is applied
/// (null clears a nullable one); an absent field is kept. Read-modify-write so
/// the cost/currency pairing is validated against the resulting values.
pub async fn update_backlink(
    State(state): State<AppState>,
    Path((org_id, backlink_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<UpdateBacklinkBody>,
) -> Result<Json<BacklinkView>, ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;

    let current = db::repositories::backlinks::get(&state.pool, &scope, backlink_id).await?;

    // Merge each field: present-in-patch wins, else keep current.
    let anchor: Option<String> = match body.expected_anchor_text {
        Some(v) => v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
        None => current.expected_anchor_text,
    };
    let rel = match body.expected_rel_type.as_deref() {
        Some(r) => {
            if !matches!(r, "dofollow" | "nofollow" | "sponsored" | "ugc") {
                return Err(ApiError::Validation(
                    "expected_rel_type must be dofollow/nofollow/sponsored/ugc".into(),
                ));
            }
            r.to_string()
        }
        None => current.expected_rel_type,
    };
    let new_cost: Option<f64> = match body.cost {
        Some(v) => v,
        None => current.cost.as_deref().and_then(|c| c.parse::<f64>().ok()),
    };
    let new_currency: Option<String> = match body.currency {
        Some(v) => v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
        None => current.currency,
    };
    if let Some(c) = new_cost
        && c < 0.0
    {
        return Err(ApiError::Validation("cost cannot be negative".into()));
    }
    if new_cost.is_some() && new_currency.is_none() {
        return Err(ApiError::Validation(
            "currency is required when cost is set".into(),
        ));
    }
    let cost_str = new_cost.map(|c| format!("{c:.2}"));

    let campaign_id = match body.campaign_id {
        Some(v) => v,
        None => current.campaign_id,
    };
    let publisher_id = match body.publisher_id {
        Some(v) => v,
        None => current.publisher_id,
    };
    // A newly-assigned campaign/publisher must belong to this org (404 else).
    if let Some(cid) = campaign_id {
        db::repositories::campaigns::get(&state.pool, &scope, cid).await?;
    }
    if let Some(pid) = publisher_id {
        db::repositories::publishers::get(&state.pool, &scope, pid).await?;
    }

    let upd = db::repositories::backlinks::BacklinkUpdate {
        expected_anchor_text: anchor.as_deref(),
        expected_rel_type: &rel,
        cost: cost_str.as_deref(),
        currency: new_currency.as_deref(),
        campaign_id,
        publisher_id,
    };
    let backlink =
        db::repositories::backlinks::update(&state.pool, &scope, backlink_id, upd).await?;
    Ok(Json(backlink.into()))
}

// ─── Bulk create (§35) ────────────────────────────────────────────────────────
const BULK_MAX: usize = 1000;

#[derive(Debug, Deserialize)]
pub struct BulkBacklinkRow {
    pub source_url: String,
    pub target_url: String,
    pub expected_anchor_text: Option<String>,
    pub expected_rel_type: Option<String>,
    pub cost: Option<f64>,
    pub currency: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BulkCreateBody {
    pub project_id: Uuid,
    pub backlinks: Vec<BulkBacklinkRow>,
}

#[derive(Debug, Serialize)]
pub struct BulkRowError {
    pub index: usize,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct BulkResult {
    pub total: usize,
    pub created: usize,
    pub duplicates: usize,
    pub errors: Vec<BulkRowError>,
}

/// POST /v1/organizations/{org}/backlinks/bulk — create many backlinks in one
/// call (§35), capped at 1000. Per-row: dedup collapses to `duplicates`, a bad
/// row goes to `errors` (never aborting the batch), and everything else is
/// created. The whole valid batch is checked against the plan cap up front.
pub async fn bulk_create_backlinks(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<BulkCreateBody>,
) -> Result<Json<BulkResult>, ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;

    if body.backlinks.is_empty() {
        return Err(ApiError::Validation("no backlinks provided".into()));
    }
    if body.backlinks.len() > BULK_MAX {
        return Err(ApiError::Validation(format!(
            "at most {BULK_MAX} backlinks per call"
        )));
    }
    // The project must belong to this org (404 otherwise).
    db::repositories::projects::get(&state.pool, &scope, body.project_id).await?;

    // Validate rows first so the plan-cap check counts only creatable ones.
    let mut errors = Vec::new();
    let mut valid = Vec::new();
    for (i, row) in body.backlinks.iter().enumerate() {
        match validate_bulk_row(row) {
            Ok(prepared) => valid.push((i, prepared)),
            Err(message) => errors.push(BulkRowError { index: i, message }),
        }
    }
    enforce_backlink_quota(&state, &scope, valid.len() as u32).await?;

    let mut created = 0usize;
    let mut duplicates = 0usize;
    for (i, p) in &valid {
        let new = db::repositories::backlinks::NewBacklink {
            project_id: body.project_id,
            campaign_id: None,
            publisher_id: None,
            source_url: &p.source_url,
            target_url: &p.target_url,
            expected_anchor_text: p.anchor.as_deref(),
            expected_rel_type: &p.rel,
            cost: p.cost.as_deref(),
            currency: p.currency.as_deref(),
            warranty_type: "none",
            warranty_value: None,
            warranty_expiration: None,
        };
        match db::repositories::backlinks::create(&state.pool, &scope, Uuid::now_v7(), new).await {
            Ok(_) => created += 1,
            Err(db::DbError::Conflict(_)) => duplicates += 1,
            Err(db::DbError::Config(_)) => errors.push(BulkRowError {
                index: *i,
                message: "could not parse a URL".into(),
            }),
            Err(e) => return Err(e.into()),
        }
    }

    Ok(Json(BulkResult {
        total: body.backlinks.len(),
        created,
        duplicates,
        errors,
    }))
}

struct PreparedRow {
    source_url: String,
    target_url: String,
    anchor: Option<String>,
    rel: String,
    cost: Option<String>,
    currency: Option<String>,
}

fn validate_bulk_row(row: &BulkBacklinkRow) -> Result<PreparedRow, String> {
    if !is_http_url(&row.source_url) || !is_http_url(&row.target_url) {
        return Err("source_url and target_url must be http(s) URLs".into());
    }
    let rel = row.expected_rel_type.as_deref().unwrap_or("dofollow");
    if !matches!(rel, "dofollow" | "nofollow" | "sponsored" | "ugc") {
        return Err("expected_rel_type must be dofollow/nofollow/sponsored/ugc".into());
    }
    let cost = match (row.cost, row.currency.as_deref()) {
        (Some(_), None) => return Err("currency is required when cost is set".into()),
        (Some(c), Some(_)) if c < 0.0 => return Err("cost cannot be negative".into()),
        (Some(c), Some(_)) => Some(format!("{c:.2}")),
        (None, _) => None,
    };
    let anchor = row
        .expected_anchor_text
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    Ok(PreparedRow {
        source_url: row.source_url.trim().to_string(),
        target_url: row.target_url.trim().to_string(),
        anchor,
        rel: rel.to_string(),
        cost,
        currency: row.currency.clone(),
    })
}

#[derive(Debug, Deserialize)]
pub struct ListBacklinksQuery {
    /// Optional now: omit to list the whole org (§35).
    pub project_id: Option<Uuid>,
    /// Filter to a client's backlinks (across its projects).
    pub client_id: Option<Uuid>,
    pub status: Option<String>,
    pub tag_id: Option<Uuid>,
    /// Free-text over source/target domain + anchor.
    pub q: Option<String>,
    pub limit: Option<i64>,
}

const BACKLINK_STATUSES: &[&str] = &[
    "pending",
    "live",
    "suspected_lost",
    "lost",
    "changed",
    "nofollow",
    "redirected",
    "source_error",
    "target_error",
    "blocked",
    "unknown",
];

pub async fn list_backlinks(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    axum::extract::Query(q): axum::extract::Query<ListBacklinksQuery>,
    jar: CookieJar,
) -> Result<Json<Vec<BacklinkView>>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;

    if let Some(status) = q.status.as_deref()
        && !BACKLINK_STATUSES.contains(&status)
    {
        return Err(ApiError::Validation("unknown status filter".into()));
    }
    let filter = db::repositories::backlinks::BacklinkFilter {
        project_id: q.project_id,
        client_id: q.client_id,
        status: q.status.as_deref(),
        tag_id: q.tag_id,
        q: q.q.as_deref(),
    };
    let rows =
        db::repositories::backlinks::search(&state.pool, &scope, &filter, q.limit.unwrap_or(50))
            .await?;
    Ok(Json(rows.into_iter().map(BacklinkView::from).collect()))
}

pub async fn get_backlink(
    State(state): State<AppState>,
    Path((org_id, backlink_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
) -> Result<Json<BacklinkView>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    let backlink = db::repositories::backlinks::get(&state.pool, &scope, backlink_id).await?;
    Ok(Json(backlink.into()))
}

/// POST /v1/organizations/{org_id}/backlinks/{backlink_id}/check — enqueue a
/// priority check (§36). Never crawls in-request (§9, §57.5): returns 202 with
/// the job id. Idempotent — repeated presses collapse to one pending job.
pub async fn check_backlink_now(
    State(state): State<AppState>,
    Path((org_id, backlink_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;

    // Confirm the backlink is in scope (404 otherwise) before enqueuing.
    db::repositories::backlinks::get(&state.pool, &scope, backlink_id).await?;

    let key = jobs::check_now_key(backlink_id);
    let payload = serde_json::json!({ "backlink_id": backlink_id });
    let job_id = db::repositories::jobs::enqueue(
        &state.pool,
        Some(scope.organization_id()),
        jobs::JobKind::CheckBacklinkNow,
        payload,
        Some(&key),
    )
    .await?;

    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({ "queued": true, "job_id": job_id })),
    ))
}

/// POST /v1/organizations/{org_id}/backlinks/{backlink_id}/archive — soft delete
/// (§36). Stops monitoring but keeps the row + history; the link drops out of
/// lists and stops counting toward the plan cap. 204 on success, 404 if not in
/// scope or already archived.
pub async fn archive_backlink(
    State(state): State<AppState>,
    Path((org_id, backlink_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    db::repositories::backlinks::archive(&state.pool, &scope, backlink_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /v1/organizations/{org_id}/backlinks/{backlink_id}/restore — undo an
/// archive and re-schedule monitoring. 204 on success, 404 if it is not archived
/// or not in scope.
pub async fn restore_backlink(
    State(state): State<AppState>,
    Path((org_id, backlink_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    db::repositories::backlinks::restore(&state.pool, &scope, backlink_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ─── Backlink events (§30) + dashboard (§34) ─────────────────────────────────
#[derive(Debug, Serialize)]
pub struct EventView {
    pub event_type: String,
    pub previous_data: Option<serde_json::Value>,
    pub current_data: Option<serde_json::Value>,
    pub detected_at: String,
}

pub async fn get_backlink_events(
    State(state): State<AppState>,
    Path((org_id, backlink_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
) -> Result<Json<Vec<EventView>>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    // Confirm the backlink is in scope first (404 otherwise).
    db::repositories::backlinks::get(&state.pool, &scope, backlink_id).await?;
    let rows =
        db::repositories::events::list_for_backlink(&state.pool, &scope, backlink_id, 200).await?;
    Ok(Json(
        rows.into_iter()
            .map(|e| EventView {
                event_type: e.event_type,
                previous_data: e.previous_data,
                current_data: e.current_data,
                detected_at: e.detected_at.to_rfc3339(),
            })
            .collect(),
    ))
}

#[derive(Debug, Serialize)]
pub struct MoneyByCurrency {
    pub currency: String,
    pub amount: String,
}

#[derive(Debug, Serialize)]
pub struct DashboardView {
    /// status -> count for every non-archived backlink.
    pub counts: std::collections::BTreeMap<String, i64>,
    pub monitored: i64,
    pub needs_attention: i64,
    pub value_at_risk: Vec<MoneyByCurrency>,
    pub tracked_spend: Vec<MoneyByCurrency>,
    pub referring_domains: i64,
    pub referring_pages: i64,
    pub occurrences: i64,
}

pub async fn dashboard(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
) -> Result<Json<DashboardView>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;

    let counts_rows = db::repositories::dashboard::status_counts(&state.pool, &scope).await?;
    let mut counts = std::collections::BTreeMap::new();
    let mut monitored = 0i64;
    let mut needs_attention = 0i64;
    const ATTENTION: &[&str] = &[
        "lost",
        "suspected_lost",
        "changed",
        "nofollow",
        "source_error",
        "target_error",
        "redirected",
    ];
    for (status, n) in counts_rows {
        monitored += n;
        if ATTENTION.contains(&status.as_str()) {
            needs_attention += n;
        }
        counts.insert(status, n);
    }

    let to_money = |rows: Vec<(String, String)>| {
        rows.into_iter()
            .map(|(currency, amount)| MoneyByCurrency { currency, amount })
            .collect::<Vec<_>>()
    };
    let value_at_risk =
        to_money(db::repositories::dashboard::value_at_risk(&state.pool, &scope).await?);
    let tracked_spend =
        to_money(db::repositories::dashboard::tracked_spend(&state.pool, &scope).await?);
    let (referring_domains, referring_pages) =
        db::repositories::dashboard::referring_counts(&state.pool, &scope).await?;
    let occurrences = db::repositories::link_occurrences::count_active(&state.pool, org_id).await?;

    Ok(Json(DashboardView {
        counts,
        monitored,
        needs_attention,
        value_at_risk,
        tracked_spend,
        referring_domains,
        referring_pages,
        occurrences,
    }))
}

#[derive(Debug, serde::Deserialize)]
pub struct EventsSummaryQuery {
    pub days: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct EventsSummaryView {
    pub since: String,
    pub days: i64,
    /// event_type -> count over the window.
    pub counts: std::collections::BTreeMap<String, i64>,
}

/// Change counts over a time window (default 30 days): new/lost/recovered,
/// follow flips, anchor/target changes, … — the dashboard "evolution" view.
pub async fn events_summary(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
    axum::extract::Query(q): axum::extract::Query<EventsSummaryQuery>,
) -> Result<Json<EventsSummaryView>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    let days = q.days.unwrap_or(30).clamp(1, 365);
    let since = chrono::Utc::now() - chrono::Duration::days(days);
    let rows = db::repositories::dashboard::event_counts(&state.pool, &scope, since).await?;
    let mut counts = std::collections::BTreeMap::new();
    for (t, n) in rows {
        counts.insert(t, n);
    }
    Ok(Json(EventsSummaryView {
        since: since.to_rfc3339(),
        days,
        counts,
    }))
}

// ─── Warranty attention (§32) ─────────────────────────────────────────────────
#[derive(Debug, Deserialize)]
pub struct ExpiringWarrantyQuery {
    /// Lookahead window in days (default 14, clamped to 0–365).
    pub within_days: Option<i64>,
}

/// Backlinks whose warranty has lapsed or lapses within the lookahead window.
/// Newest-lapsing first; each carries its computed `warranty_status`.
pub async fn expiring_warranties(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    axum::extract::Query(q): axum::extract::Query<ExpiringWarrantyQuery>,
    jar: CookieJar,
) -> Result<Json<Vec<BacklinkView>>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    let within = q.within_days.unwrap_or(WARRANTY_SOON_DAYS).clamp(0, 365);
    let cutoff = chrono::Utc::now().date_naive() + chrono::Duration::days(within);
    let rows =
        db::repositories::backlinks::warranty_expiring_for_org(&state.pool, &scope, cutoff, 200)
            .await?;
    Ok(Json(rows.into_iter().map(BacklinkView::from).collect()))
}

// ─── Notifications feed (§37) ─────────────────────────────────────────────────
#[derive(Debug, Serialize)]
pub struct NotificationView {
    pub id: Uuid,
    pub kind: String,
    pub title: String,
    pub body: Option<String>,
    pub backlink_id: Option<Uuid>,
    pub read: bool,
    pub created_at: String,
}

impl From<db::entities::Notification> for NotificationView {
    fn from(n: db::entities::Notification) -> Self {
        NotificationView {
            id: n.id,
            kind: n.kind,
            title: n.title,
            body: n.body,
            backlink_id: n.backlink_id,
            read: n.read_at.is_some(),
            created_at: n.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct NotificationsResponse {
    pub items: Vec<NotificationView>,
    pub unread: i64,
}

pub async fn list_notifications(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
) -> Result<Json<NotificationsResponse>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    let rows = db::repositories::notifications::list(&state.pool, &scope, 100).await?;
    let unread = db::repositories::notifications::unread_count(&state.pool, &scope).await?;
    Ok(Json(NotificationsResponse {
        items: rows.into_iter().map(NotificationView::from).collect(),
        unread,
    }))
}

pub async fn read_notification(
    State(state): State<AppState>,
    Path((org_id, notification_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    db::repositories::notifications::mark_read(&state.pool, &scope, notification_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ─── CSV export (§36) ─────────────────────────────────────────────────────────
#[derive(Debug, Deserialize)]
pub struct ExportQuery {
    /// Optional project filter; omitted exports the whole org.
    pub project_id: Option<Uuid>,
    /// Optional client filter (across the client's projects).
    pub client_id: Option<Uuid>,
}

/// Stream a CSV of the org's backlinks. Cells are guarded against CSV/formula
/// injection (docs/11 S8) — a crawled anchor like `=cmd` cannot execute in a
/// spreadsheet. The header row matches the importer's generic layout, so an
/// export re-imports; extra columns (status, warranty, last check) are ignored
/// on import.
pub async fn export_backlinks(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    axum::extract::Query(q): axum::extract::Query<ExportQuery>,
    jar: CookieJar,
) -> Result<axum::response::Response, ApiError> {
    use axum::http::header;
    use axum::response::IntoResponse;

    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    let rows = db::repositories::backlinks::list_for_export(
        &state.pool,
        &scope,
        q.project_id,
        q.client_id,
        50_000,
    )
    .await?;

    // Map each backlink's project -> its client name (project.client_id -> client).
    let client_name: std::collections::HashMap<Uuid, String> =
        db::repositories::clients::list(&state.pool, &scope)
            .await?
            .into_iter()
            .map(|c| (c.id, c.name))
            .collect();
    let project_client: std::collections::HashMap<Uuid, String> =
        db::repositories::projects::list(&state.pool, &scope)
            .await?
            .into_iter()
            .filter_map(|p| {
                p.client_id
                    .and_then(|cid| client_name.get(&cid).cloned())
                    .map(|name| (p.id, name))
            })
            .collect();

    const HEADERS: &[&str] = &[
        "client",
        "source_url",
        "target_url",
        "anchor",
        "rel",
        "cost",
        "currency",
        "status",
        "warranty_type",
        "warranty_value",
        "warranty_expiration",
        "last_checked_at",
    ];
    let records: Vec<Vec<String>> = rows
        .into_iter()
        .map(|b| {
            vec![
                project_client
                    .get(&b.project_id)
                    .cloned()
                    .unwrap_or_default(),
                b.source_url,
                b.target_url,
                b.expected_anchor_text.unwrap_or_default(),
                b.expected_rel_type,
                b.cost.unwrap_or_default(),
                b.currency.unwrap_or_default(),
                b.status,
                b.warranty_type,
                b.warranty_value.map(|v| v.to_string()).unwrap_or_default(),
                b.warranty_expiration
                    .map(|d| d.to_string())
                    .unwrap_or_default(),
                b.last_checked_at
                    .map(|t| t.to_rfc3339())
                    .unwrap_or_default(),
            ]
        })
        .collect();

    let csv = backlio_core::csv_export::write_csv(HEADERS, &records);
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"backlinks.csv\"",
            ),
        ],
        csv,
    )
        .into_response())
}

// ─── CSV import (§38) ────────────────────────────────────────────────────────
#[derive(Debug, Deserialize)]
pub struct ImportBody {
    pub project_id: Uuid,
    /// Raw CSV text (generic / Ahrefs / Semrush headers).
    pub csv: String,
}

#[derive(Debug, Serialize)]
pub struct RowError {
    pub row_number: usize,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ImportResult {
    pub total: usize,
    pub imported: usize,
    pub duplicates: usize,
    pub errors: Vec<RowError>,
}

/// Synchronous bulk import for reasonably-sized CSVs (§38). Large files move to
/// an async chunked job later (docs/07 §5); for now a row cap keeps it bounded.
pub async fn import_csv(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<ImportBody>,
) -> Result<Json<ImportResult>, ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    // Project must be in scope (404 otherwise).
    db::repositories::projects::get(&state.pool, &scope, body.project_id).await?;

    let parsed = backlio_core::csv_import::parse_csv(&body.csv).map_err(ApiError::Validation)?;
    if parsed.total > 5000 {
        return Err(ApiError::Validation(
            "more than 5000 rows — split the file (async import for large files is coming)".into(),
        ));
    }
    // Enforce the plan cap for the whole batch of valid rows (§39).
    let valid_count = parsed.valid().count() as u32;
    enforce_backlink_quota(&state, &scope, valid_count).await?;

    let mut imported = 0usize;
    let mut duplicates = 0usize;
    let mut errors = Vec::new();
    for row in parsed.rows {
        match row {
            backlio_core::csv_import::RowResult::Error {
                row_number,
                message,
            } => {
                errors.push(RowError {
                    row_number,
                    message,
                });
            }
            backlio_core::csv_import::RowResult::Ok(r) => {
                let new = db::repositories::backlinks::NewBacklink {
                    project_id: body.project_id,
                    campaign_id: None,
                    publisher_id: None,
                    source_url: &r.source_url,
                    target_url: &r.target_url,
                    expected_anchor_text: r.anchor.as_deref(),
                    expected_rel_type: &r.rel_type,
                    cost: r.cost.as_deref(),
                    currency: r.currency.as_deref(),
                    warranty_type: "none",
                    warranty_value: None,
                    warranty_expiration: None,
                };
                match db::repositories::backlinks::create(&state.pool, &scope, Uuid::now_v7(), new)
                    .await
                {
                    Ok(_) => imported += 1,
                    Err(db::DbError::Conflict(_)) => duplicates += 1,
                    Err(db::DbError::Config(_)) => errors.push(RowError {
                        row_number: r.row_number,
                        message: "could not parse a URL".into(),
                    }),
                    Err(e) => return Err(e.into()),
                }
            }
        }
    }
    Ok(Json(ImportResult {
        total: parsed.total,
        imported,
        duplicates,
        errors,
    }))
}
