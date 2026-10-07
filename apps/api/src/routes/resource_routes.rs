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
use db::repositories::{backlinks, clients, projects, tags};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Reject the request when the workspace has no active paid access — an expired
/// trial or an inactive/canceled subscription (docs/20). Existing data is kept;
/// only new paid actions are blocked.
async fn enforce_active_access(state: &AppState, scope: &db::OrgScope) -> Result<(), ApiError> {
    if db::repositories::billing::is_access_active(&state.pool, scope.organization_id()).await? {
        Ok(())
    } else {
        Err(ApiError::PlanLimit(
            "your trial has ended or your subscription is inactive — choose a plan to continue"
                .into(),
        ))
    }
}

/// Enforce the org's plan backlink cap before adding `count` (§39). Existing
/// links are never touched; only new adds over the cap are blocked.
async fn enforce_backlink_quota(
    state: &AppState,
    scope: &db::OrgScope,
    count: u32,
) -> Result<(), ApiError> {
    enforce_active_access(state, scope).await?;
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
    // Deletes are admin+ only (docs/05 §2 role matrix); members can create/edit
    // but not delete clients/projects/backlinks.
    scope.require(Role::Admin)?;
    clients::delete(&state.pool, &scope, client_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// PATCH a client — full-form edit of name + contact/company fields.
pub async fn update_client(
    State(state): State<AppState>,
    Path((org_id, client_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<CreateClientBody>,
) -> Result<Json<ClientView>, ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;

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
    let client = clients::update(&state.pool, &scope, client_id, name, extra).await?;
    Ok(Json(client.into()))
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
    enforce_active_access(&state, &scope).await?;
    // Plan project limit (docs/20). Over-limit blocks new projects; existing kept.
    let plan =
        db::repositories::billing::plan_for_org(&state.pool, scope.organization_id()).await?;
    let current = projects::count(&state.pool, &scope).await? as u32;
    if let billing::AddOutcome::OverLimit { limit, .. } =
        billing::Entitlements::new(plan).can_add_project(current)
    {
        return Err(ApiError::PlanLimit(format!(
            "plan limit of {limit} projects reached — upgrade to add more"
        )));
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

/// DELETE a project and all its data (backlinks, campaigns, seeds, competitors
/// cascade). Irreversible.
pub async fn delete_project(
    State(state): State<AppState>,
    Path((org_id, project_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    // Deletes are admin+ only (docs/05 §2). Deleting a project cascades to its
    // backlinks, campaigns, seeds and competitors — a destructive, admin-level op.
    scope.require(Role::Admin)?;
    projects::delete(&state.pool, &scope, project_id).await?;
    Ok(StatusCode::NO_CONTENT)
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
    pub tier: i16,
    pub parent_backlink_id: Option<Uuid>,
    // ─── Manual classification (§ link classification, migration 0034) ────────
    /// How the link was obtained (slug, null = unclassified).
    pub acquisition: Option<String>,
    /// Where on the page the link sits (slug, null = unclassified).
    pub placement: Option<String>,
    /// The content format (slug, null = unclassified).
    pub link_format: Option<String>,
    /// Manual attribute override (follow/nofollow/sponsored/ugc); null = use the
    /// crawler-detected one below.
    pub link_attribute: Option<String>,
    /// Crawler-derived attribute (follow/nofollow/sponsored/ugc); null if the
    /// link has never been successfully observed.
    pub link_attribute_auto: Option<String>,
    // ─── Selective monitoring (migration 0035) ────────────────────────────────
    pub monitoring_enabled: bool,
    pub monitoring_level: String,
    pub check_frequency: Option<String>,
    pub monitoring_reason: Option<String>,
    pub monitoring_overridden: bool,
    pub next_check_at: Option<String>,
    // ─── Economic / commercial ────────────────────────────────────────────────
    pub commercial_model: Option<String>,
    pub order_reference: Option<String>,
    pub purchase_date: Option<String>,
    pub renewal_date: Option<String>,
    pub replacement_eligible: Option<bool>,
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
        let link_attribute_auto = derived_attribute(rel.as_deref(), b.detected_is_dofollow);
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
            tier: b.tier,
            parent_backlink_id: b.parent_backlink_id,
            acquisition: b.acquisition,
            placement: b.placement,
            link_format: b.link_format,
            link_attribute: b.link_attribute,
            link_attribute_auto,
            monitoring_enabled: b.monitoring_enabled,
            monitoring_level: b.monitoring_level,
            check_frequency: b.check_frequency,
            monitoring_reason: b.monitoring_reason,
            monitoring_overridden: b.monitoring_overridden,
            next_check_at: b.next_check_at.map(|d| d.to_rfc3339()),
            commercial_model: b.commercial_model,
            order_reference: b.order_reference,
            purchase_date: b.purchase_date.map(|d| d.to_string()),
            renewal_date: b.renewal_date.map(|d| d.to_string()),
            replacement_eligible: b.replacement_eligible,
        }
    }
}

// ─── Link classification vocabularies (migration 0034) ────────────────────────
// Stored as slugs; the web maps them to display labels. Kept in code so the
// vocabulary can evolve without a migration (the columns are plain text).
const ACQUISITIONS: &[&str] = &[
    "paid",
    "exchange",
    "earned",
    "outreach",
    "partnership",
    "self_created",
    "other",
];
const LINK_ATTRIBUTES: &[&str] = &["follow", "nofollow", "sponsored", "ugc"];
const PLACEMENTS: &[&str] = &[
    "in_content",
    "author_bio",
    "homepage",
    "sidebar",
    "footer",
    "navigation",
    "resource_list",
    "image",
    "widget",
    "sitewide",
];
const LINK_FORMATS: &[&str] = &[
    "guest_post",
    "niche_edit",
    "editorial",
    "pr",
    "directory",
    "forum",
];

// ─── Selective monitoring vocabularies (migration 0035) ───────────────────────
const MONITORING_LEVELS: &[&str] = &["protected", "tracked", "inventory_only"];
const CHECK_FREQUENCIES: &[&str] = &["daily", "weekly", "biweekly", "monthly", "manual"];
const COMMERCIAL_MODELS: &[&str] = &[
    "one_time",
    "rental",
    "recurring",
    "exchange",
    "partnership",
    "free",
    "unknown",
];

/// Merge a PATCH date field (YYYY-MM-DD string): present -> parsed (empty -> clear),
/// absent -> keep current.
fn merge_date(
    field: &Option<Option<String>>,
    current: Option<chrono::NaiveDate>,
    name: &str,
) -> Result<Option<chrono::NaiveDate>, ApiError> {
    match field {
        Some(v) => match v.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
            Some(s) => chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .map(Some)
                .map_err(|_| ApiError::Validation(format!("{name} must be YYYY-MM-DD"))),
            None => Ok(None),
        },
        None => Ok(current),
    }
}

/// Monitoring decision for a bulk/import row (no acquisition or warranty context):
/// organic by default, monitored only when it carries a positive cost.
fn monitoring_decision_for_row(cost: Option<&str>) -> backlio_core::MonitoringDecision {
    let has_cost = cost
        .and_then(|s| s.parse::<f64>().ok())
        .is_some_and(|c| c > 0.0);
    backlio_core::monitoring::decide(backlio_core::MonitoringInputs {
        acquisition: None,
        has_cost,
        under_warranty: false,
        subject: "own",
        tier: 1,
    })
}

/// Interpret a classification filter query param. The sentinel `"none"` means
/// "unclassified" (match rows with no value); any other value is validated
/// against `allowed`; absent means no filter. Returns (value, unclassified).
fn classify_filter<'a>(
    v: Option<&'a str>,
    allowed: &[&str],
    name: &str,
) -> Result<(Option<&'a str>, bool), ApiError> {
    match v {
        None => Ok((None, false)),
        Some("none") => Ok((None, true)),
        Some(val) if allowed.contains(&val) => Ok((Some(val), false)),
        Some(_) => Err(ApiError::Validation(format!("unknown {name} filter"))),
    }
}

/// The crawler-derived attribute slug (follow/nofollow/sponsored/ugc), or None
/// if the link has never been successfully observed. Mirrors the SQL fallback in
/// `backlinks::search` so manual filters and the displayed value agree.
fn derived_attribute(rel: Option<&[String]>, is_dofollow: Option<bool>) -> Option<String> {
    let has = |t: &str| rel.map(|a| a.iter().any(|r| r == t)).unwrap_or(false);
    if has("sponsored") {
        Some("sponsored".into())
    } else if has("ugc") {
        Some("ugc".into())
    } else if has("nofollow") || is_dofollow == Some(false) {
        Some("nofollow".into())
    } else if is_dofollow == Some(true) {
        Some("follow".into())
    } else {
        None
    }
}

/// Merge one classification dimension for a PATCH: present-in-body (trimmed,
/// empty → clear) validated against `allowed`, else keep `current`.
fn merge_classification(
    field: &Option<Option<String>>,
    current: Option<String>,
    allowed: &[&str],
    name: &str,
) -> Result<Option<String>, ApiError> {
    match field {
        Some(v) => {
            let val = v
                .as_ref()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            if let Some(ref s) = val
                && !allowed.contains(&s.as_str())
            {
                return Err(ApiError::Validation(format!("unknown {name} value")));
            }
            Ok(val)
        }
        None => Ok(current),
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

/// GET the Tier-2 backlinks reinforcing a Tier-1 backlink (§9).
pub async fn list_tier2(
    State(state): State<AppState>,
    Path((org_id, backlink_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
) -> Result<Json<Vec<BacklinkView>>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Viewer)?;
    db::repositories::backlinks::get(&state.pool, &scope, backlink_id).await?;
    let rows =
        db::repositories::backlinks::search_tier2(&state.pool, &scope, backlink_id, 500).await?;
    Ok(Json(rows.into_iter().map(BacklinkView::from).collect()))
}

#[derive(Debug, serde::Deserialize)]
pub struct Tier2Row {
    pub source_url: String,
    pub anchor: Option<String>,
    pub rel: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Tier2ImportBody {
    pub rows: Vec<Tier2Row>,
}

#[derive(Debug, Serialize)]
pub struct Tier2ImportResult {
    pub imported: usize,
    pub new: usize,
    pub skipped: usize,
}

/// Import Tier-2 backlinks that reinforce a Tier-1 backlink (§8, §21 Import).
pub async fn import_tier2(
    State(state): State<AppState>,
    Path((org_id, backlink_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<Tier2ImportBody>,
) -> Result<Json<Tier2ImportResult>, ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    let parent = db::repositories::backlinks::get(&state.pool, &scope, backlink_id).await?;

    let (mut imported, mut new, mut skipped) = (0usize, 0usize, 0usize);
    for row in body.rows.into_iter().take(5_000) {
        let rel = row.rel.as_deref().unwrap_or("dofollow");
        if !is_http_url(&row.source_url)
            || !matches!(rel, "dofollow" | "nofollow" | "sponsored" | "ugc")
        {
            skipped += 1;
            continue;
        }
        match db::repositories::tiers::record_tier2(
            &state.pool,
            &scope,
            &parent,
            row.source_url.trim(),
            row.anchor
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty()),
            rel,
        )
        .await
        {
            Ok((_, is_new)) => {
                imported += 1;
                if is_new {
                    new += 1;
                }
            }
            Err(_) => skipped += 1,
        }
    }
    Ok(Json(Tier2ImportResult {
        imported,
        new,
        skipped,
    }))
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
    /// Explicit monitoring choice from the UI (the "Monitor these links" box,
    /// checked by default). `Some(true)` forces monitoring on even for a link the
    /// policy would leave as inventory; `Some(false)` forces inventory-only;
    /// `None` keeps the automatic economic/contractual policy.
    pub monitor: Option<bool>,
}

/// Apply the UI's explicit monitor choice on top of the automatic policy
/// `decision`. Returns (enabled, level, reason).
fn resolve_monitoring(
    decision: &backlio_core::MonitoringDecision,
    monitor: Option<bool>,
) -> (bool, &'static str, Option<&'static str>) {
    match monitor {
        Some(true) if !decision.enabled => (true, "tracked", Some("user_opt_in")),
        Some(false) => (false, "inventory_only", None),
        _ => (
            decision.enabled,
            decision.level.as_str(),
            decision.reason.map(|r| r.as_str()),
        ),
    }
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
    // Selective monitoring (§ monitoring policy): a new link is only scheduled
    // when it has an economic/contractual reason; everything else is inventory.
    let decision = backlio_core::monitoring::decide(backlio_core::MonitoringInputs {
        acquisition: None,
        has_cost: body.cost.is_some_and(|c| c > 0.0),
        under_warranty: warranty_kind != backlio_core::WarrantyType::None,
        subject: "own",
        tier: 1,
    });
    let (mon_enabled, mon_level, mon_reason) = resolve_monitoring(&decision, body.monitor);
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
        monitoring_enabled: mon_enabled,
        monitoring_level: mon_level,
        monitoring_reason: mon_reason,
        check_frequency: None,
    };
    let backlink = db::repositories::backlinks::create(&state.pool, &scope, Uuid::now_v7(), new)
        .await
        .map_err(|e| match e {
            db::DbError::Config(_) => ApiError::Validation("could not parse a URL".into()),
            other => other.into(),
        })?;
    // Funnel: the first monitored backlink activates the workspace. The count==1
    // check keeps this exactly-once (best-effort — docs/20 §P8).
    if let Ok(1) = db::repositories::backlinks::count_active(&state.pool, &scope).await {
        let _ = db::repositories::analytics::record(
            &state.pool,
            "activated",
            Some(user.id),
            Some(scope.organization_id()),
            None,
            serde_json::json!({ "via": "manual" }),
        )
        .await;
    }
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
    // Classification (migration 0034); each keep/clear via de_double_opt.
    #[serde(default, deserialize_with = "de_double_opt")]
    pub acquisition: Option<Option<String>>,
    #[serde(default, deserialize_with = "de_double_opt")]
    pub link_attribute: Option<Option<String>>,
    #[serde(default, deserialize_with = "de_double_opt")]
    pub placement: Option<Option<String>>,
    #[serde(default, deserialize_with = "de_double_opt")]
    pub link_format: Option<Option<String>>,
    // Monitoring + economic (migration 0035). Setting any monitoring field marks
    // the link as a manual override (the policy will not touch it again).
    pub monitoring_enabled: Option<bool>,
    pub monitoring_level: Option<String>,
    #[serde(default, deserialize_with = "de_double_opt")]
    pub check_frequency: Option<Option<String>>,
    #[serde(default, deserialize_with = "de_double_opt")]
    pub commercial_model: Option<Option<String>>,
    #[serde(default, deserialize_with = "de_double_opt")]
    pub order_reference: Option<Option<String>>,
    #[serde(default, deserialize_with = "de_double_opt")]
    pub purchase_date: Option<Option<String>>,
    #[serde(default, deserialize_with = "de_double_opt")]
    pub renewal_date: Option<Option<String>>,
    #[serde(default, deserialize_with = "de_double_opt")]
    pub replacement_eligible: Option<Option<bool>>,
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

    // Classification dimensions (validated against their vocabularies).
    let acquisition = merge_classification(
        &body.acquisition,
        current.acquisition,
        ACQUISITIONS,
        "acquisition",
    )?;
    let link_attribute = merge_classification(
        &body.link_attribute,
        current.link_attribute,
        LINK_ATTRIBUTES,
        "link attribute",
    )?;
    let placement =
        merge_classification(&body.placement, current.placement, PLACEMENTS, "placement")?;
    let link_format = merge_classification(
        &body.link_format,
        current.link_format,
        LINK_FORMATS,
        "format",
    )?;

    // Monitoring (migration 0035). Any monitoring field in the body is a manual
    // override that wins over the default policy from then on.
    let monitoring_enabled = body
        .monitoring_enabled
        .unwrap_or(current.monitoring_enabled);
    let monitoring_level = match body.monitoring_level.as_deref() {
        Some(l) => {
            if !MONITORING_LEVELS.contains(&l) {
                return Err(ApiError::Validation("unknown monitoring level".into()));
            }
            l.to_string()
        }
        None => current.monitoring_level,
    };
    let check_frequency = match &body.check_frequency {
        Some(v) => {
            let f = v
                .as_ref()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            if let Some(ref s) = f
                && !CHECK_FREQUENCIES.contains(&s.as_str())
            {
                return Err(ApiError::Validation("unknown check frequency".into()));
            }
            f
        }
        None => current.check_frequency,
    };
    let monitoring_touched = body.monitoring_enabled.is_some()
        || body.monitoring_level.is_some()
        || body.check_frequency.is_some();
    let monitoring_overridden = current.monitoring_overridden || monitoring_touched;
    let monitoring_reason: Option<String> = if monitoring_touched {
        Some("manual".to_string())
    } else {
        current.monitoring_reason
    };

    // Economic (migration 0035).
    let commercial_model = merge_classification(
        &body.commercial_model,
        current.commercial_model,
        COMMERCIAL_MODELS,
        "commercial model",
    )?;
    let order_reference = match &body.order_reference {
        Some(v) => v
            .as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        None => current.order_reference,
    };
    let purchase_date = merge_date(&body.purchase_date, current.purchase_date, "purchase_date")?;
    let renewal_date = merge_date(&body.renewal_date, current.renewal_date, "renewal_date")?;
    let replacement_eligible = match body.replacement_eligible {
        Some(v) => v,
        None => current.replacement_eligible,
    };

    let upd = db::repositories::backlinks::BacklinkUpdate {
        expected_anchor_text: anchor.as_deref(),
        expected_rel_type: &rel,
        cost: cost_str.as_deref(),
        currency: new_currency.as_deref(),
        campaign_id,
        publisher_id,
        acquisition: acquisition.as_deref(),
        link_attribute: link_attribute.as_deref(),
        placement: placement.as_deref(),
        link_format: link_format.as_deref(),
        monitoring_enabled,
        monitoring_level: &monitoring_level,
        monitoring_reason: monitoring_reason.as_deref(),
        check_frequency: check_frequency.as_deref(),
        monitoring_overridden,
        commercial_model: commercial_model.as_deref(),
        order_reference: order_reference.as_deref(),
        purchase_date,
        renewal_date,
        replacement_eligible,
    };
    let backlink =
        db::repositories::backlinks::update(&state.pool, &scope, backlink_id, upd).await?;
    Ok(Json(backlink.into()))
}

// ─── Bulk monitoring changes (§15, migration 0035) ───────────────────────────
const BULK_MONITORING_MAX: usize = 5000;

#[derive(Debug, Deserialize)]
pub struct BulkMonitoringBody {
    pub ids: Vec<Uuid>,
    /// Any subset may be set; omitted fields are left unchanged.
    pub monitoring_enabled: Option<bool>,
    pub monitoring_level: Option<String>,
    pub check_frequency: Option<String>,
}

#[derive(Serialize)]
pub struct BulkMonitoringResult {
    pub updated: i64,
}

/// POST …/backlinks/bulk-monitoring — change monitoring on many links at once
/// (§15). Marks each row as a manual override and re-derives its schedule, so
/// disabling a set immediately stops its future checks.
pub async fn bulk_monitoring(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<BulkMonitoringBody>,
) -> Result<Json<BulkMonitoringResult>, ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;

    if body.ids.is_empty() {
        return Err(ApiError::Validation("no backlink ids provided".into()));
    }
    if body.ids.len() > BULK_MONITORING_MAX {
        return Err(ApiError::Validation(format!(
            "at most {BULK_MONITORING_MAX} ids per call"
        )));
    }
    if let Some(l) = body.monitoring_level.as_deref()
        && !MONITORING_LEVELS.contains(&l)
    {
        return Err(ApiError::Validation("unknown monitoring level".into()));
    }
    if let Some(f) = body.check_frequency.as_deref()
        && !CHECK_FREQUENCIES.contains(&f)
    {
        return Err(ApiError::Validation("unknown check frequency".into()));
    }
    if body.monitoring_enabled.is_none()
        && body.monitoring_level.is_none()
        && body.check_frequency.is_none()
    {
        return Err(ApiError::Validation("nothing to change".into()));
    }
    let updated = db::repositories::backlinks::bulk_set_monitoring(
        &state.pool,
        &scope,
        &body.ids,
        body.monitoring_enabled,
        body.monitoring_level.as_deref(),
        body.check_frequency.as_deref(),
    )
    .await?;
    Ok(Json(BulkMonitoringResult {
        updated: updated as i64,
    }))
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
    /// Explicit monitoring choice (the "Monitor these links" box). See
    /// `CreateBacklinkBody::monitor`. `None` keeps the automatic policy per row.
    pub monitor: Option<bool>,
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
        let decision = monitoring_decision_for_row(p.cost.as_deref());
        let (mon_enabled, mon_level, mon_reason) = resolve_monitoring(&decision, body.monitor);
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
            monitoring_enabled: mon_enabled,
            monitoring_level: mon_level,
            monitoring_reason: mon_reason,
            check_frequency: None,
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
    /// Pagination offset (rows to skip); 0/absent = first page.
    pub offset: Option<i64>,
    /// Acquisition window in days (first-seen); omit/0 = all time.
    pub days: Option<i64>,
    // Classification filters (migration 0034); each a single slug.
    pub acquisition: Option<String>,
    /// Matches the EFFECTIVE attribute (manual override, else crawler-derived).
    pub attribute: Option<String>,
    pub placement: Option<String>,
    pub format: Option<String>,
    /// Monitoring tab filter: protected / tracked / inventory_only.
    pub monitoring_level: Option<String>,
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
    // Each classification filter: a real value, the "none" sentinel
    // (unclassified), or absent.
    let (acq, acq_unset) = classify_filter(q.acquisition.as_deref(), ACQUISITIONS, "acquisition")?;
    let (attr, attr_unset) = classify_filter(q.attribute.as_deref(), LINK_ATTRIBUTES, "attribute")?;
    let (place, place_unset) = classify_filter(q.placement.as_deref(), PLACEMENTS, "placement")?;
    let (fmt, fmt_unset) = classify_filter(q.format.as_deref(), LINK_FORMATS, "format")?;
    if let Some(level) = q.monitoring_level.as_deref()
        && !MONITORING_LEVELS.contains(&level)
    {
        return Err(ApiError::Validation(
            "unknown monitoring level filter".into(),
        ));
    }
    let since = q
        .days
        .filter(|d| *d > 0)
        .map(|d| chrono::Utc::now() - chrono::Duration::days(d));
    let filter = db::repositories::backlinks::BacklinkFilter {
        project_id: q.project_id,
        client_id: q.client_id,
        status: q.status.as_deref(),
        tag_id: q.tag_id,
        q: q.q.as_deref(),
        offset: q.offset.unwrap_or(0).max(0),
        since,
        acquisition: acq,
        link_attribute: attr,
        placement: place,
        link_format: fmt,
        acquisition_unset: acq_unset,
        link_attribute_unset: attr_unset,
        placement_unset: place_unset,
        link_format_unset: fmt_unset,
        monitoring_level: q.monitoring_level.as_deref(),
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
    // Archiving is the soft-delete for a backlink (there is no hard delete), so
    // it is "delete backlinks" per docs/05 §2 — admin+ only, not members.
    scope.require(Role::Admin)?;
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
    // Restore reverses an admin's archive, so it stays at the same admin+ level
    // as archive (a member should not override an admin's delete).
    scope.require(Role::Admin)?;
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
    // Selective monitoring (migration 0035): counts by level + protected value.
    pub protected: i64,
    pub tracked: i64,
    pub inventory_only: i64,
    pub value_protected: Vec<MoneyByCurrency>,
    pub value_at_risk: Vec<MoneyByCurrency>,
    pub tracked_spend: Vec<MoneyByCurrency>,
    pub referring_domains: i64,
    pub referring_pages: i64,
    pub occurrences: i64,
    /// [dofollow, nofollow] over the selected period.
    pub rel: [i64; 2],
    /// link-type tag -> count (comment / pbn / forum-profile / self-built / earned).
    pub link_types: std::collections::BTreeMap<String, i64>,
}

#[derive(Debug, Deserialize)]
pub struct DashboardQuery {
    /// Acquisition window in days (first-seen); omit/0 = all time.
    pub days: Option<i64>,
}

pub async fn dashboard(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    axum::extract::Query(q): axum::extract::Query<DashboardQuery>,
    jar: CookieJar,
) -> Result<Json<DashboardView>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;

    // Acquisition period (first-seen) filter for the counts + illustrations.
    let since = q
        .days
        .filter(|d| *d > 0)
        .map(|d| chrono::Utc::now() - chrono::Duration::days(d));

    let counts_rows =
        db::repositories::dashboard::status_counts(&state.pool, &scope, since).await?;
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
    let value_protected =
        to_money(db::repositories::dashboard::value_protected(&state.pool, &scope).await?);
    let (mut protected, mut tracked, mut inventory_only) = (0i64, 0i64, 0i64);
    for (level, n) in
        db::repositories::dashboard::monitoring_counts(&state.pool, &scope, since).await?
    {
        match level.as_str() {
            "protected" => protected = n,
            "tracked" => tracked = n,
            _ => inventory_only += n,
        }
    }
    let (referring_domains, referring_pages) =
        db::repositories::dashboard::referring_counts(&state.pool, &scope).await?;
    let occurrences = db::repositories::link_occurrences::count_active(&state.pool, org_id).await?;

    let (dofollow, nofollow) =
        db::repositories::dashboard::rel_split(&state.pool, &scope, since).await?;
    let link_types = db::repositories::dashboard::type_counts(&state.pool, &scope, since)
        .await?
        .into_iter()
        .collect::<std::collections::BTreeMap<_, _>>();

    Ok(Json(DashboardView {
        counts,
        monitored,
        needs_attention,
        protected,
        tracked,
        inventory_only,
        value_protected,
        value_at_risk,
        tracked_spend,
        referring_domains,
        referring_pages,
        occurrences,
        rel: [dofollow, nofollow],
        link_types,
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
                let decision = monitoring_decision_for_row(r.cost.as_deref());
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
                    monitoring_enabled: decision.enabled,
                    monitoring_level: decision.level.as_str(),
                    monitoring_reason: decision.reason.map(|r| r.as_str()),
                    check_frequency: None,
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
    // Funnel: if this import added the workspace's first backlinks, it's now
    // activated (count == imported means it was empty before — docs/20 §P8).
    if imported > 0
        && let Ok(n) = db::repositories::backlinks::count_active(&state.pool, &scope).await
        && n as usize == imported
    {
        let _ = db::repositories::analytics::record(
            &state.pool,
            "activated",
            Some(user.id),
            Some(scope.organization_id()),
            None,
            serde_json::json!({ "via": "csv_import" }),
        )
        .await;
    }
    Ok(Json(ImportResult {
        total: parsed.total,
        imported,
        duplicates,
        errors,
    }))
}

// ─── Footprint auto-tagger (§35) ─────────────────────────────────────────────
// Classify a project's own backlinks by footprint and attach a category tag, so
// self-placed / PBN / forum-profile links are told apart from earned ones at a
// glance. Heuristic and reversible (the user can retag or detach). Idempotent:
// re-running only adds missing attachments.

const AUTOTAG_TAGS: &[(&str, &str)] = &[
    ("comment", "#22d3ee"),
    ("pbn", "#fb7185"),
    ("forum-profile", "#a78bfa"),
    ("self-built", "#fbbf24"),
    ("earned", "#36d399"),
];

// Links dropped in a blog/article comment section ("ninja linking"). The tell is
// in the source URL, not the anchor: WordPress/Disqus and most comment engines
// point at a comment fragment or a reply query param. Placement is the defining
// footprint here, so this is checked before anchor/domain heuristics.
fn is_comment_link(source_url: &str) -> bool {
    let u = source_url.to_lowercase();
    const MARKERS: &[&str] = &[
        "#comment",    // #comment-12345, #comments (WordPress, generic)
        "#respond",    // WordPress reply form anchor
        "#disqus",     // #disqus_thread
        "#li-comment", // some themes
        "#reply",      // #reply-title / #reply-12
        "replytocom=", // ?replytocom= / &replytocom= (WordPress)
        "/comment-page-",
        "/comments/",
    ];
    MARKERS.iter().any(|m| u.contains(m))
}

fn classify_footprint(
    anchor: &str,
    domain: &str,
    source_url: &str,
    domain_count: usize,
) -> &'static str {
    let a = anchor.to_lowercase();
    let d = domain.to_lowercase();
    // Comment-placed link (ninja linking) — most specific footprint, so first.
    if is_comment_link(source_url) {
        return "comment";
    }
    // Promotional / PBN anchor text (bulk-built or bought).
    const PROMO: &[&str] = &[
        "premium",
        "pbn",
        "dofollow backlink",
        "link building",
        "outreach",
        "rank higher",
        "buy backlink",
        "da 50",
        "pa 40",
        "high quality",
        "backlinks da",
        "seo strateg",
    ];
    if PROMO.iter().any(|k| a.contains(k)) {
        return "pbn";
    }
    // Many links from one domain → sitewide / forum profile / signature.
    if domain_count >= 8 {
        return "forum-profile";
    }
    // Spammy TLDs or SEO-keyword domains → self-built / auto network.
    const SPAM_TLD: &[&str] = &[
        ".link", ".shop", ".store", ".site", ".online", ".info", ".website", ".space", ".click",
        ".xyz",
    ];
    const SEO_KW: &[&str] = &[
        "seo", "backlink", "checker", "boost", "traffic", "rank", "dapa", "casino",
    ];
    if SPAM_TLD.iter().any(|t| d.ends_with(t)) || SEO_KW.iter().any(|k| d.contains(k)) {
        return "self-built";
    }
    "earned"
}

#[derive(Serialize)]
pub struct AutoTagResult {
    pub total: usize,
    pub counts: HashMap<String, usize>,
}

pub async fn auto_tag_backlinks(
    State(state): State<AppState>,
    Path((org_id, project_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
) -> Result<Json<AutoTagResult>, ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    projects::get(&state.pool, &scope, project_id).await?; // 404 if not in scope

    let rows = backlinks::list_for_autotag(&state.pool, &scope, project_id).await?;

    // Links-per-domain, to spot sitewide / forum-profile footprints.
    let mut domain_count: HashMap<&str, usize> = HashMap::new();
    for r in &rows {
        *domain_count.entry(r.source_domain.as_str()).or_default() += 1;
    }

    // Get-or-create the category tags (idempotent across re-runs).
    let existing = tags::list(&state.pool, &scope).await?;
    let mut tag_ids: HashMap<&str, Uuid> = HashMap::new();
    for (name, color) in AUTOTAG_TAGS {
        let id = match existing.iter().find(|t| t.name.eq_ignore_ascii_case(name)) {
            Some(t) => t.id,
            None => {
                tags::create(&state.pool, &scope, Uuid::now_v7(), name, Some(color))
                    .await?
                    .id
            }
        };
        tag_ids.insert(name, id);
    }

    let mut counts: HashMap<String, usize> = HashMap::new();
    for r in &rows {
        let n = *domain_count.get(r.source_domain.as_str()).unwrap_or(&1);
        let cat = classify_footprint(&r.anchor, &r.source_domain, &r.source_url, n);
        if let Some(&tid) = tag_ids.get(cat) {
            let _ = tags::attach(&state.pool, &scope, r.id, tid).await;
        }
        *counts.entry(cat.to_string()).or_default() += 1;
    }

    Ok(Json(AutoTagResult {
        total: rows.len(),
        counts,
    }))
}
