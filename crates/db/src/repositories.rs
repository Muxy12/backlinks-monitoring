//! Repositories for the identity/tenancy slice (docs/03, docs/05). Tenant-owned
//! reads take an [`OrgScope`](crate::scope::OrgScope) so a query cannot forget
//! its tenant filter (ADR-0004). The identity tables (`users`, `sessions`) are
//! not tenant-owned and are keyed by their own ids.
//!
//! Queries are runtime `sqlx::query_as`, so they need no compile-time database
//! or `.sqlx` cache. sqlx 0.9 requires the SQL to be a `&'static str` literal
//! (a guard against injection), so every statement below is a literal — column
//! lists are spelled out rather than interpolated.

use crate::entities::{Organization, Session, User};
use crate::error::DbError;
use crate::scope::OrgScope;
use chrono::{DateTime, Utc};
use common::Role;
use sqlx::PgPool;
use uuid::Uuid;

// ─── Users ───────────────────────────────────────────────────────────────────
pub mod users {
    use super::*;

    pub async fn create(
        pool: &PgPool,
        id: Uuid,
        email: &str,
        password_hash: &str,
        name: &str,
    ) -> Result<User, DbError> {
        sqlx::query_as::<_, User>(
            "INSERT INTO users (id, email, password_hash, name) \
             VALUES ($1, $2, $3, $4) \
             RETURNING id, email, password_hash, name, email_verified_at, created_at, updated_at",
        )
        .bind(id)
        .bind(email)
        .bind(password_hash)
        .bind(name)
        .fetch_one(pool)
        .await
        .map_err(DbError::from_insert)
    }

    pub async fn find_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, DbError> {
        Ok(sqlx::query_as::<_, User>(
            "SELECT id, email, password_hash, name, email_verified_at, created_at, updated_at \
             FROM users WHERE email = $1",
        )
        .bind(email)
        .fetch_optional(pool)
        .await?)
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<User, DbError> {
        sqlx::query_as::<_, User>(
            "SELECT id, email, password_hash, name, email_verified_at, created_at, updated_at \
             FROM users WHERE id = $1",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(DbError::from_fetch)
    }

    pub async fn mark_email_verified(pool: &PgPool, id: Uuid) -> Result<(), DbError> {
        sqlx::query(
            "UPDATE users SET email_verified_at = now() \
             WHERE id = $1 AND email_verified_at IS NULL",
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }
}

// ─── Organizations + membership ──────────────────────────────────────────────
pub mod orgs {
    use super::*;

    /// Create an org and make `owner_user_id` its `owner`, atomically — an org
    /// must always have at least one owner (docs/05 §2).
    pub async fn create_with_owner(
        pool: &PgPool,
        org_id: Uuid,
        name: &str,
        slug: &str,
        owner_user_id: Uuid,
    ) -> Result<Organization, DbError> {
        let mut tx = pool.begin().await?;

        let org = sqlx::query_as::<_, Organization>(
            "INSERT INTO organizations (id, name, slug) VALUES ($1, $2, $3) \
             RETURNING id, name, slug, created_at, updated_at",
        )
        .bind(org_id)
        .bind(name)
        .bind(slug)
        .fetch_one(&mut *tx)
        .await
        .map_err(DbError::from_insert)?;

        sqlx::query(
            "INSERT INTO organization_members (organization_id, user_id, role) \
             VALUES ($1, $2, 'owner')",
        )
        .bind(org_id)
        .bind(owner_user_id)
        .execute(&mut *tx)
        .await
        .map_err(DbError::from_insert)?;

        tx.commit().await?;
        Ok(org)
    }

    /// Fetch an org the caller is scoped to. Scoped by the verified `OrgScope`,
    /// so this cannot return another tenant's row.
    pub async fn get(pool: &PgPool, scope: &OrgScope) -> Result<Organization, DbError> {
        sqlx::query_as::<_, Organization>(
            "SELECT id, name, slug, created_at, updated_at FROM organizations WHERE id = $1",
        )
        .bind(scope.organization_id())
        .fetch_one(pool)
        .await
        .map_err(DbError::from_fetch)
    }

    /// Every org a user belongs to, with their role — the `/me` org list.
    pub async fn list_for_user(
        pool: &PgPool,
        user_id: Uuid,
    ) -> Result<Vec<(Organization, Role)>, DbError> {
        let rows =
            sqlx::query_as::<_, (Uuid, String, String, DateTime<Utc>, DateTime<Utc>, String)>(
                "SELECT o.id, o.name, o.slug, o.created_at, o.updated_at, m.role::text AS role \
             FROM organizations o \
             JOIN organization_members m ON m.organization_id = o.id \
             WHERE m.user_id = $1 \
             ORDER BY o.created_at",
            )
            .bind(user_id)
            .fetch_all(pool)
            .await?;

        rows.into_iter()
            .map(|(id, name, slug, created_at, updated_at, role)| {
                let role = role
                    .parse::<Role>()
                    .map_err(|_| DbError::Config(format!("unknown role {role:?}")))?;
                Ok((
                    Organization {
                        id,
                        name,
                        slug,
                        created_at,
                        updated_at,
                    },
                    role,
                ))
            })
            .collect()
    }
}

// ─── Sessions ────────────────────────────────────────────────────────────────
pub mod sessions {
    use super::*;

    pub async fn create(
        pool: &PgPool,
        id: Uuid,
        user_id: Uuid,
        token_hash: &[u8],
        csrf_token_hash: &[u8],
        expires_at: DateTime<Utc>,
    ) -> Result<Session, DbError> {
        sqlx::query_as::<_, Session>(
            "INSERT INTO sessions (id, user_id, token_hash, csrf_token_hash, expires_at) \
             VALUES ($1, $2, $3, $4, $5) \
             RETURNING id, user_id, token_hash, csrf_token_hash, expires_at, revoked_at",
        )
        .bind(id)
        .bind(user_id)
        .bind(token_hash)
        .bind(csrf_token_hash)
        .bind(expires_at)
        .fetch_one(pool)
        .await
        .map_err(DbError::from_insert)
    }

    /// Look up a live session by the token's hash. Returns `NotFound` when
    /// missing, revoked, or expired — callers cannot tell which.
    pub async fn find_active_by_token_hash(
        pool: &PgPool,
        token_hash: &[u8],
        now: DateTime<Utc>,
    ) -> Result<Session, DbError> {
        sqlx::query_as::<_, Session>(
            "SELECT id, user_id, token_hash, csrf_token_hash, expires_at, revoked_at \
             FROM sessions WHERE token_hash = $1 AND revoked_at IS NULL AND expires_at > $2",
        )
        .bind(token_hash)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(DbError::from_fetch)
    }

    pub async fn revoke(pool: &PgPool, id: Uuid) -> Result<(), DbError> {
        sqlx::query("UPDATE sessions SET revoked_at = now() WHERE id = $1 AND revoked_at IS NULL")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Revoke every session for a user — used on password reset (S4, docs/09 §2).
    pub async fn revoke_all_for_user(pool: &PgPool, user_id: Uuid) -> Result<u64, DbError> {
        let r = sqlx::query(
            "UPDATE sessions SET revoked_at = now() WHERE user_id = $1 AND revoked_at IS NULL",
        )
        .bind(user_id)
        .execute(pool)
        .await?;
        Ok(r.rows_affected())
    }
}

// ─── Clients (§20) — tenant-scoped ───────────────────────────────────────────
pub mod clients {
    use super::*;
    use crate::entities::Client;

    pub async fn create(
        pool: &PgPool,
        scope: &OrgScope,
        id: Uuid,
        name: &str,
    ) -> Result<Client, DbError> {
        sqlx::query_as::<_, Client>(
            "INSERT INTO clients (id, organization_id, name) VALUES ($1, $2, $3) \
             RETURNING id, organization_id, name, company_name, website, \
                       contact_name, contact_email, notes, created_at, updated_at",
        )
        .bind(id)
        .bind(scope.organization_id())
        .bind(name)
        .fetch_one(pool)
        .await
        .map_err(DbError::from_insert)
    }

    pub async fn list(pool: &PgPool, scope: &OrgScope) -> Result<Vec<Client>, DbError> {
        Ok(sqlx::query_as::<_, Client>(
            "SELECT id, organization_id, name, company_name, website, \
                    contact_name, contact_email, notes, created_at, updated_at \
             FROM clients WHERE organization_id = $1 ORDER BY name",
        )
        .bind(scope.organization_id())
        .fetch_all(pool)
        .await?)
    }

    /// Fetch by id, always filtered by the scoped org. A cross-tenant id yields
    /// `NotFound`, never another tenant's row (ADR-0004).
    pub async fn get(pool: &PgPool, scope: &OrgScope, id: Uuid) -> Result<Client, DbError> {
        sqlx::query_as::<_, Client>(
            "SELECT id, organization_id, name, company_name, website, \
                    contact_name, contact_email, notes, created_at, updated_at \
             FROM clients WHERE id = $1 AND organization_id = $2",
        )
        .bind(id)
        .bind(scope.organization_id())
        .fetch_one(pool)
        .await
        .map_err(DbError::from_fetch)
    }
}

// ─── Projects (§21) — tenant-scoped ──────────────────────────────────────────
pub mod projects {
    // Frequency read as seconds, status read as text, so no custom type mapping.
    use super::*;
    use crate::entities::Project;

    pub async fn create(
        pool: &PgPool,
        scope: &OrgScope,
        id: Uuid,
        name: &str,
        domain: &str,
        check_frequency_secs: i64,
    ) -> Result<Project, DbError> {
        sqlx::query_as::<_, Project>(
            "WITH inserted AS ( \
                INSERT INTO projects (id, organization_id, name, domain, default_check_frequency) \
                VALUES ($1, $2, $3, $4, make_interval(secs => $5)) \
                RETURNING * \
             ) \
             SELECT id, organization_id, client_id, name, domain, \
                    EXTRACT(EPOCH FROM default_check_frequency)::bigint AS check_frequency_secs, \
                    timezone, status::text AS status, created_at, updated_at, archived_at \
             FROM inserted",
        )
        .bind(id)
        .bind(scope.organization_id())
        .bind(name)
        .bind(domain)
        .bind(check_frequency_secs as f64)
        .fetch_one(pool)
        .await
        .map_err(DbError::from_insert)
    }

    pub async fn list(pool: &PgPool, scope: &OrgScope) -> Result<Vec<Project>, DbError> {
        Ok(sqlx::query_as::<_, Project>(
            "SELECT id, organization_id, client_id, name, domain, \
                    EXTRACT(EPOCH FROM default_check_frequency)::bigint AS check_frequency_secs, \
                    timezone, status::text AS status, created_at, updated_at, archived_at \
             FROM projects WHERE organization_id = $1 ORDER BY created_at",
        )
        .bind(scope.organization_id())
        .fetch_all(pool)
        .await?)
    }

    pub async fn get(pool: &PgPool, scope: &OrgScope, id: Uuid) -> Result<Project, DbError> {
        sqlx::query_as::<_, Project>(
            "SELECT id, organization_id, client_id, name, domain, \
                    EXTRACT(EPOCH FROM default_check_frequency)::bigint AS check_frequency_secs, \
                    timezone, status::text AS status, created_at, updated_at, archived_at \
             FROM projects WHERE id = $1 AND organization_id = $2",
        )
        .bind(id)
        .bind(scope.organization_id())
        .fetch_one(pool)
        .await
        .map_err(DbError::from_fetch)
    }
}

// ─── Backlinks (§23) — tenant-scoped ─────────────────────────────────────────
pub mod backlinks {
    use super::*;
    use crate::entities::Backlink;
    use backlio_core::{NormalizeOptions, host_of, normalize};

    // Shared RETURNING/SELECT projection (literal for sqlx 0.9).
    macro_rules! cols {
        () => {
            "id, organization_id, project_id, campaign_id, publisher_id, \
             source_url, source_url_normalized, source_domain, \
             target_url, target_url_normalized, target_domain, \
             expected_anchor_text, expected_rel_type::text AS expected_rel_type, \
             status::text AS status, cost::text AS cost, currency, \
             warranty_type::text AS warranty_type, warranty_value, warranty_expiration, \
             created_at, updated_at, next_check_at, last_checked_at"
        };
    }

    /// Input for creating a backlink. URLs are normalized here (docs/03 §3.4)
    /// via the same normalizer the crawler uses, so a stored link and a
    /// discovered link compare identically.
    pub struct NewBacklink<'a> {
        pub project_id: Uuid,
        pub campaign_id: Option<Uuid>,
        pub publisher_id: Option<Uuid>,
        pub source_url: &'a str,
        pub target_url: &'a str,
        pub expected_anchor_text: Option<&'a str>,
        pub expected_rel_type: &'a str,
        pub cost: Option<&'a str>,
        pub currency: Option<&'a str>,
        /// Warranty is stored as `(type, value)`; the expiration date is
        /// materialized by the caller (via `backlio_core::warranty_expiration`)
        /// so the DB can index it (docs/03 §4.3, §32).
        pub warranty_type: &'a str,
        pub warranty_value: Option<i32>,
        pub warranty_expiration: Option<chrono::NaiveDate>,
    }

    pub async fn create(
        pool: &PgPool,
        scope: &OrgScope,
        id: Uuid,
        new: NewBacklink<'_>,
    ) -> Result<Backlink, DbError> {
        let opts = NormalizeOptions::default();
        let source_norm = normalize(new.source_url, None, opts)
            .ok_or_else(|| DbError::Config("invalid source_url".into()))?;
        let target_norm = normalize(new.target_url, None, opts)
            .ok_or_else(|| DbError::Config("invalid target_url".into()))?;
        let source_domain = host_of(&source_norm).unwrap_or_default();
        let target_domain = host_of(&target_norm).unwrap_or_default();

        // next_check_at = now() so a new backlink is immediately due for its
        // first check via the scheduler (§49 "schedule initial monitoring").
        sqlx::query_as::<_, Backlink>(concat!(
            "INSERT INTO backlinks (id, organization_id, project_id, \
                campaign_id, publisher_id, \
                source_url, source_url_normalized, source_domain, \
                target_url, target_url_normalized, target_domain, \
                expected_anchor_text, expected_rel_type, cost, currency, \
                warranty_type, warranty_value, warranty_expiration, next_check_at) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13::rel_type,$14::numeric,$15, \
                     $16::warranty_type,$17,$18, now()) \
             RETURNING ",
            cols!()
        ))
        .bind(id)
        .bind(scope.organization_id())
        .bind(new.project_id)
        .bind(new.campaign_id)
        .bind(new.publisher_id)
        .bind(new.source_url)
        .bind(&source_norm)
        .bind(&source_domain)
        .bind(new.target_url)
        .bind(&target_norm)
        .bind(&target_domain)
        .bind(new.expected_anchor_text)
        .bind(new.expected_rel_type)
        .bind(new.cost)
        .bind(new.currency)
        .bind(new.warranty_type)
        .bind(new.warranty_value)
        .bind(new.warranty_expiration)
        .fetch_one(pool)
        .await
        .map_err(DbError::from_insert)
    }

    /// List a project's backlinks (newest first). Tenant + project scoped.
    /// Pagination refinement (keyset) lands with the filter work (§35).
    pub async fn list_for_project(
        pool: &PgPool,
        scope: &OrgScope,
        project_id: Uuid,
        limit: i64,
    ) -> Result<Vec<Backlink>, DbError> {
        Ok(sqlx::query_as::<_, Backlink>(concat!(
            "SELECT ",
            cols!(),
            " FROM backlinks WHERE organization_id = $1 AND project_id = $2 \
              AND archived_at IS NULL ORDER BY created_at DESC LIMIT $3"
        ))
        .bind(scope.organization_id())
        .bind(project_id)
        .bind(limit.clamp(1, 200))
        .fetch_all(pool)
        .await?)
    }

    pub async fn get(pool: &PgPool, scope: &OrgScope, id: Uuid) -> Result<Backlink, DbError> {
        sqlx::query_as::<_, Backlink>(concat!(
            "SELECT ",
            cols!(),
            " FROM backlinks WHERE id = $1 AND organization_id = $2"
        ))
        .bind(id)
        .bind(scope.organization_id())
        .fetch_one(pool)
        .await
        .map_err(DbError::from_fetch)
    }

    /// Count active backlinks in an org — for plan-limit enforcement (§39).
    pub async fn count_active(pool: &PgPool, scope: &OrgScope) -> Result<i64, DbError> {
        let (n,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM backlinks WHERE organization_id = $1 AND archived_at IS NULL",
        )
        .bind(scope.organization_id())
        .fetch_one(pool)
        .await?;
        Ok(n)
    }

    /// One org's warranted links whose warranty lapses on or before `cutoff`
    /// (today + a lookahead window) — for the dashboard's "warranty attention"
    /// list (§32). Only `days`/`months` warranties have a date; `lifetime`/`none`
    /// never appear. Newest-lapsing first.
    pub async fn warranty_expiring_for_org(
        pool: &PgPool,
        scope: &OrgScope,
        cutoff: chrono::NaiveDate,
        limit: i64,
    ) -> Result<Vec<Backlink>, DbError> {
        Ok(sqlx::query_as::<_, Backlink>(concat!(
            "SELECT ",
            cols!(),
            " FROM backlinks \
              WHERE organization_id = $1 AND archived_at IS NULL \
                AND warranty_expiration IS NOT NULL AND warranty_expiration <= $2 \
              ORDER BY warranty_expiration LIMIT $3"
        ))
        .bind(scope.organization_id())
        .bind(cutoff)
        .bind(limit.clamp(1, 500))
        .fetch_all(pool)
        .await?)
    }
}

// ─── Jobs (durable queue enqueue side) — docs/07 ─────────────────────────────
pub mod jobs {
    use super::*;
    use ::jobs::JobKind;
    use serde_json::Value;

    /// Enqueue a job. Idempotent when `idempotency_key` is set: a second enqueue
    /// while an equivalent job is still pending/processing is a no-op (the
    /// partial unique index `jobs_idempotency_uq`), so "Check now" x5 -> one job
    /// (§36, docs/07 §4). Returns the new job id, or `None` when deduped.
    pub async fn enqueue(
        pool: &PgPool,
        organization_id: Option<Uuid>,
        kind: JobKind,
        payload: Value,
        idempotency_key: Option<&str>,
    ) -> Result<Option<Uuid>, DbError> {
        let row: Option<(Uuid,)> = sqlx::query_as(
            "INSERT INTO jobs (id, organization_id, job_type, payload, priority, idempotency_key) \
             VALUES ($1, $2, $3, $4, $5, $6) \
             ON CONFLICT DO NOTHING \
             RETURNING id",
        )
        .bind(Uuid::now_v7())
        .bind(organization_id)
        .bind(kind.as_str())
        .bind(payload)
        .bind(kind.priority())
        .bind(idempotency_key)
        .fetch_optional(pool)
        .await?;
        Ok(row.map(|(id,)| id))
    }

    /// Count pending jobs of a kind — for observability/tests.
    pub async fn count_pending(pool: &PgPool, kind: JobKind) -> Result<i64, DbError> {
        let (n,): (i64,) =
            sqlx::query_as("SELECT count(*) FROM jobs WHERE job_type = $1 AND status = 'pending'")
                .bind(kind.as_str())
                .fetch_one(pool)
                .await?;
        Ok(n)
    }
}

// ─── Monitoring: state load + atomic result persistence (docs/07, docs/08) ────
pub mod monitor {
    use super::*;
    use crate::entities::MonitorState;
    use serde_json::Value;

    /// Load the fields the state machine needs for one backlink.
    pub async fn load_state(pool: &PgPool, id: Uuid) -> Result<MonitorState, DbError> {
        sqlx::query_as::<_, MonitorState>(
            "SELECT id, organization_id, status::text AS status, \
                    expected_anchor_text, expected_rel_type::text AS expected_rel_type, \
                    detected_anchor_text, detected_target_url, detected_rel_attributes, \
                    detected_is_dofollow, source_canonical, source_robots_noindex, \
                    consecutive_failure_count, last_seen_at, source_url, target_url \
             FROM backlinks WHERE id = $1",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(DbError::from_fetch)
    }

    /// The new state to persist after a check. Enum-ish fields are strings that
    /// the SQL casts to their Postgres enum types.
    pub struct TransitionWrite {
        pub status: String,
        pub previous_status: String,
        pub consecutive_failure_count: i32,
        pub next_check_at: chrono::DateTime<Utc>,
        pub last_seen_at: Option<chrono::DateTime<Utc>>,
        pub detected_anchor_text: Option<String>,
        pub detected_target_url: Option<String>,
        pub detected_rel_attributes: Option<Vec<String>>,
        pub detected_is_dofollow: Option<bool>,
        pub source_canonical: Option<String>,
        pub source_robots_noindex: bool,
        pub source_http_status: Option<i32>,
    }

    /// One check row to append.
    pub struct CheckWrite {
        pub crawl_id: Uuid,
        pub found: bool,
        pub status_after: String,
        pub source_status: Option<i32>,
        pub anchor_text: Option<String>,
        pub rel_attributes: Option<Vec<String>>,
        pub is_dofollow: Option<bool>,
        pub final_url: Option<String>,
        pub duration_ms: Option<i32>,
        pub bytes: Option<i32>,
        pub error_kind: Option<String>,
    }

    /// One event to append (type + before/after JSON).
    pub struct EventWrite {
        pub event_type: String,
        pub previous_data: Option<Value>,
        pub current_data: Option<Value>,
    }

    /// Persist a check, its events, and the backlink's new state — atomically
    /// (docs/07 §4, docs/08 §6). Idempotent on `(backlink_id, crawl_id)`: a
    /// re-run with the same crawl id writes no second check row and skips the
    /// update (returns Ok with `applied = false`).
    pub async fn apply_check(
        pool: &PgPool,
        organization_id: Uuid,
        backlink_id: Uuid,
        check: CheckWrite,
        transition: TransitionWrite,
        events: Vec<EventWrite>,
    ) -> Result<bool, DbError> {
        let mut tx = pool.begin().await?;

        let check_id = Uuid::now_v7();
        let inserted: Option<(Uuid,)> = sqlx::query_as(
            "INSERT INTO backlink_checks (id, backlink_id, organization_id, crawl_id, found, \
                status_after, source_status, anchor_text, rel_attributes, is_dofollow, \
                final_url, duration_ms, bytes, error_kind) \
             VALUES ($1,$2,$3,$4,$5,$6::backlink_status,$7,$8,$9,$10,$11,$12,$13,$14) \
             ON CONFLICT (backlink_id, crawl_id) DO NOTHING \
             RETURNING id",
        )
        .bind(check_id)
        .bind(backlink_id)
        .bind(organization_id)
        .bind(check.crawl_id)
        .bind(check.found)
        .bind(&check.status_after)
        .bind(check.source_status)
        .bind(&check.anchor_text)
        .bind(&check.rel_attributes)
        .bind(check.is_dofollow)
        .bind(&check.final_url)
        .bind(check.duration_ms)
        .bind(check.bytes)
        .bind(&check.error_kind)
        .fetch_optional(&mut *tx)
        .await
        .map_err(DbError::from_insert)?;

        // Already processed this crawl for this backlink -> no-op (idempotent).
        let Some((check_id,)) = inserted else {
            tx.rollback().await?;
            return Ok(false);
        };

        for ev in &events {
            sqlx::query(
                "INSERT INTO backlink_events (id, backlink_id, organization_id, check_id, \
                    event_type, previous_data, current_data) \
                 VALUES ($1,$2,$3,$4,$5::event_type,$6,$7)",
            )
            .bind(Uuid::now_v7())
            .bind(backlink_id)
            .bind(organization_id)
            .bind(check_id)
            .bind(&ev.event_type)
            .bind(&ev.previous_data)
            .bind(&ev.current_data)
            .execute(&mut *tx)
            .await
            .map_err(DbError::from_insert)?;
        }

        sqlx::query(
            "UPDATE backlinks SET \
                status = $2::backlink_status, previous_status = $3::backlink_status, \
                consecutive_failure_count = $4, next_check_at = $5, last_seen_at = $6, \
                detected_anchor_text = $7, detected_target_url = $8, detected_rel_attributes = $9, \
                detected_is_dofollow = $10, source_canonical = $11, source_robots_noindex = $12, \
                source_http_status = $13, last_checked_at = now(), \
                first_checked_at = COALESCE(first_checked_at, now()), \
                failure_count = failure_count + $14 \
             WHERE id = $1",
        )
        .bind(backlink_id)
        .bind(&transition.status)
        .bind(&transition.previous_status)
        .bind(transition.consecutive_failure_count)
        .bind(transition.next_check_at)
        .bind(transition.last_seen_at)
        .bind(&transition.detected_anchor_text)
        .bind(&transition.detected_target_url)
        .bind(&transition.detected_rel_attributes)
        .bind(transition.detected_is_dofollow)
        .bind(&transition.source_canonical)
        .bind(transition.source_robots_noindex)
        .bind(transition.source_http_status)
        .bind(if check.found { 0_i32 } else { 1_i32 })
        .execute(&mut *tx)
        .await
        .map_err(DbError::from_insert)?;

        tx.commit().await?;
        Ok(true)
    }
}

// ─── Job queue: claim / complete / fail / reap (docs/07 §2–§3) ───────────────
pub mod job_queue {
    use super::*;
    use serde_json::Value;

    #[derive(Debug, Clone, sqlx::FromRow)]
    pub struct ClaimedJob {
        pub id: Uuid,
        pub organization_id: Option<Uuid>,
        pub job_type: String,
        pub payload: Value,
        pub attempts: i32,
        pub max_attempts: i32,
    }

    /// Atomically claim up to `limit` due jobs with FOR UPDATE SKIP LOCKED
    /// (docs/07 §2). No worker blocks another; no job goes to two workers.
    pub async fn claim(
        pool: &PgPool,
        worker_id: &str,
        limit: i64,
    ) -> Result<Vec<ClaimedJob>, DbError> {
        Ok(sqlx::query_as::<_, ClaimedJob>(
            "UPDATE jobs SET status='processing', locked_at=now(), locked_by=$1, \
                    attempts=attempts+1, updated_at=now() \
             WHERE id IN ( \
                SELECT id FROM jobs WHERE status='pending' AND scheduled_at <= now() \
                ORDER BY priority DESC, scheduled_at FOR UPDATE SKIP LOCKED LIMIT $2 \
             ) RETURNING id, organization_id, job_type, payload, attempts, max_attempts",
        )
        .bind(worker_id)
        .bind(limit)
        .fetch_all(pool)
        .await?)
    }

    pub async fn complete(pool: &PgPool, id: Uuid) -> Result<(), DbError> {
        sqlx::query(
            "UPDATE jobs SET status='completed', completed_at=now(), updated_at=now() WHERE id=$1",
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Mark a job failed: reschedule for retry, or move to `dead` when the error
    /// is permanent or attempts are exhausted (docs/07 §3).
    pub async fn fail(
        pool: &PgPool,
        id: Uuid,
        error: &str,
        retry_in_secs: Option<i64>,
    ) -> Result<(), DbError> {
        match retry_in_secs {
            Some(secs) => {
                sqlx::query(
                    "UPDATE jobs SET status='pending', scheduled_at = now() + make_interval(secs => $2), \
                            last_error=$3, locked_at=NULL, locked_by=NULL, updated_at=now() WHERE id=$1",
                )
                .bind(id)
                .bind(secs as f64)
                .bind(error)
                .execute(pool)
                .await?;
            }
            None => {
                sqlx::query(
                    "UPDATE jobs SET status='dead', failed_at=now(), last_error=$2, updated_at=now() WHERE id=$1",
                )
                .bind(id)
                .bind(error)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    /// Return jobs stuck in `processing` past their lease to `pending` (docs/07
    /// §3.1). Rows past max_attempts go to `dead`. Returns rows reaped.
    pub async fn reap(pool: &PgPool, lease_secs: i64) -> Result<u64, DbError> {
        let requeued = sqlx::query(
            "UPDATE jobs SET status='pending', locked_at=NULL, locked_by=NULL, updated_at=now() \
             WHERE status='processing' AND locked_at < now() - make_interval(secs => $1) \
               AND attempts < max_attempts",
        )
        .bind(lease_secs as f64)
        .execute(pool)
        .await?;
        sqlx::query(
            "UPDATE jobs SET status='dead', failed_at=now(), last_error='reaped: max attempts', updated_at=now() \
             WHERE status='processing' AND locked_at < now() - make_interval(secs => $1) \
               AND attempts >= max_attempts",
        )
        .bind(lease_secs as f64)
        .execute(pool)
        .await?;
        Ok(requeued.rows_affected())
    }
}

// ─── Scheduler: enqueue due checks, deduped by source URL (docs/07 §6) ───────
pub mod scheduler {
    use super::*;
    use ::jobs::JobKind;
    use std::collections::BTreeMap;

    /// Transaction-scoped advisory lock so only one scheduler runs at a time.
    /// `pg_advisory_xact_lock` works through Neon's transaction pooler, unlike
    /// session-level locks (docs/01 §2.2).
    const LOCK_KEY: i64 = 4_200_042;

    /// Find due backlinks, group them by normalized source URL, enqueue one
    /// `check_source_page` job per group, and bump `next_check_at` on the claimed
    /// rows by `lease_secs` so they are not re-enqueued before the checks run —
    /// all in one transaction (docs/07 §6). Returns (jobs enqueued, backlinks
    /// scheduled).
    pub async fn enqueue_due(
        pool: &PgPool,
        batch: i64,
        lease_secs: i64,
    ) -> Result<(usize, usize), DbError> {
        let mut tx = pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(LOCK_KEY)
            .execute(&mut *tx)
            .await?;

        let due: Vec<(Uuid, String, String)> = sqlx::query_as(
            "SELECT id, source_url, source_url_normalized FROM backlinks \
             WHERE archived_at IS NULL AND next_check_at IS NOT NULL AND next_check_at <= now() \
             ORDER BY next_check_at LIMIT $1 FOR UPDATE SKIP LOCKED",
        )
        .bind(batch)
        .fetch_all(&mut *tx)
        .await?;

        if due.is_empty() {
            tx.commit().await?;
            return Ok((0, 0));
        }

        // Group by normalized source URL; keep one representative raw URL to
        // fetch (any in the group normalizes the same, §27).
        let mut groups: BTreeMap<String, (String, Vec<Uuid>)> = BTreeMap::new();
        let mut all_ids: Vec<Uuid> = Vec::with_capacity(due.len());
        for (id, src, norm) in &due {
            all_ids.push(*id);
            groups
                .entry(norm.clone())
                .or_insert_with(|| (src.clone(), Vec::new()))
                .1
                .push(*id);
        }

        for (src, ids) in groups.values() {
            let payload = serde_json::json!({ "source_url": src, "backlink_ids": ids });
            sqlx::query("INSERT INTO jobs (id, job_type, payload, priority) VALUES ($1,$2,$3,$4)")
                .bind(Uuid::now_v7())
                .bind(JobKind::CheckSourcePage.as_str())
                .bind(payload)
                .bind(JobKind::CheckSourcePage.priority())
                .execute(&mut *tx)
                .await?;
        }

        sqlx::query(
            "UPDATE backlinks SET next_check_at = now() + make_interval(secs => $2), updated_at = now() \
             WHERE id = ANY($1)",
        )
        .bind(&all_ids)
        .bind(lease_secs as f64)
        .execute(&mut *tx)
        .await?;

        let jobs_enqueued = groups.len();
        tx.commit().await?;
        Ok((jobs_enqueued, all_ids.len()))
    }
}

// ─── Backlink events / timeline (§30) ────────────────────────────────────────
pub mod events {
    use super::*;
    use crate::entities::BacklinkEvent;

    /// The event timeline for one backlink, newest first. Tenant-scoped.
    pub async fn list_for_backlink(
        pool: &PgPool,
        scope: &OrgScope,
        backlink_id: Uuid,
        limit: i64,
    ) -> Result<Vec<BacklinkEvent>, DbError> {
        Ok(sqlx::query_as::<_, BacklinkEvent>(
            "SELECT id, backlink_id, event_type::text AS event_type, previous_data, current_data, detected_at \
             FROM backlink_events WHERE organization_id = $1 AND backlink_id = $2 \
             ORDER BY detected_at DESC LIMIT $3",
        )
        .bind(scope.organization_id())
        .bind(backlink_id)
        .bind(limit.clamp(1, 500))
        .fetch_all(pool)
        .await?)
    }
}

// ─── Dashboard summary (§33, §34) ────────────────────────────────────────────
pub mod dashboard {
    use super::*;

    /// (status, count) for every non-archived backlink in the org.
    pub async fn status_counts(
        pool: &PgPool,
        scope: &OrgScope,
    ) -> Result<Vec<(String, i64)>, DbError> {
        Ok(sqlx::query_as::<_, (String, i64)>(
            "SELECT status::text, count(*) FROM backlinks \
             WHERE organization_id = $1 AND archived_at IS NULL GROUP BY status",
        )
        .bind(scope.organization_id())
        .fetch_all(pool)
        .await?)
    }

    /// (currency, sum(cost)) for links needing attention — Value at Risk (§33).
    /// Grouped by currency; never summed across (that would be a lie).
    pub async fn value_at_risk(
        pool: &PgPool,
        scope: &OrgScope,
    ) -> Result<Vec<(String, String)>, DbError> {
        Ok(sqlx::query_as::<_, (String, String)>(
            "SELECT currency, sum(cost)::text FROM backlinks \
             WHERE organization_id = $1 AND archived_at IS NULL AND cost IS NOT NULL \
               AND status IN ('lost','changed','nofollow','source_error','target_error','redirected') \
             GROUP BY currency",
        )
        .bind(scope.organization_id())
        .fetch_all(pool)
        .await?)
    }

    /// (currency, sum(cost)) for all tracked spend (§34).
    pub async fn tracked_spend(
        pool: &PgPool,
        scope: &OrgScope,
    ) -> Result<Vec<(String, String)>, DbError> {
        Ok(sqlx::query_as::<_, (String, String)>(
            "SELECT currency, sum(cost)::text FROM backlinks \
             WHERE organization_id = $1 AND archived_at IS NULL AND cost IS NOT NULL \
             GROUP BY currency",
        )
        .bind(scope.organization_id())
        .fetch_all(pool)
        .await?)
    }
}

// ─── Billing: plan lookup + idempotent webhook event record (§39) ────────────
pub mod billing {
    use super::*;
    use ::billing::Plan;

    /// The org's active plan, defaulting to Starter when there is no active
    /// subscription (a new/free org still monitors up to the Starter cap).
    pub async fn plan_for_org(pool: &PgPool, organization_id: Uuid) -> Result<Plan, DbError> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT plan::text FROM subscriptions \
             WHERE organization_id = $1 AND status IN ('trialing','active','past_due')",
        )
        .bind(organization_id)
        .fetch_optional(pool)
        .await?;
        Ok(row
            .and_then(|(p,)| Plan::from_slug(&p))
            .unwrap_or(Plan::Starter))
    }

    /// Record a Stripe event id, idempotently (§39, S5). Returns true if this is
    /// the first time we've seen it (should process), false on replay (no-op).
    pub async fn record_stripe_event(
        pool: &PgPool,
        event_id: &str,
        event_type: &str,
    ) -> Result<bool, DbError> {
        let row: Option<(String,)> = sqlx::query_as(
            "INSERT INTO stripe_events (event_id, event_type) VALUES ($1, $2) \
             ON CONFLICT (event_id) DO NOTHING RETURNING event_id",
        )
        .bind(event_id)
        .bind(event_type)
        .fetch_optional(pool)
        .await?;
        Ok(row.is_some())
    }
}

// ─── Campaigns (§22) — tenant-scoped, with read-time rollups ──────────────────
pub mod campaigns {
    use super::*;
    use crate::entities::{Campaign, RollupCounts};

    // status/budget cast to text so no custom sqlx type mapping is needed.
    macro_rules! cols {
        () => {
            "id, organization_id, project_id, name, description, \
             status::text AS status, start_date, end_date, target_backlinks, \
             budget::text AS budget, currency, created_at, updated_at"
        };
    }

    pub struct NewCampaign<'a> {
        pub project_id: Uuid,
        pub name: &'a str,
        pub description: Option<&'a str>,
        pub target_backlinks: Option<i32>,
        pub budget: Option<&'a str>,
        pub currency: Option<&'a str>,
    }

    pub async fn create(
        pool: &PgPool,
        scope: &OrgScope,
        id: Uuid,
        new: NewCampaign<'_>,
    ) -> Result<Campaign, DbError> {
        sqlx::query_as::<_, Campaign>(concat!(
            "INSERT INTO campaigns \
                (id, organization_id, project_id, name, description, \
                 target_backlinks, budget, currency) \
             VALUES ($1,$2,$3,$4,$5,$6,$7::numeric,$8) \
             RETURNING ",
            cols!()
        ))
        .bind(id)
        .bind(scope.organization_id())
        .bind(new.project_id)
        .bind(new.name)
        .bind(new.description)
        .bind(new.target_backlinks)
        .bind(new.budget)
        .bind(new.currency)
        .fetch_one(pool)
        .await
        .map_err(DbError::from_insert)
    }

    pub async fn list(pool: &PgPool, scope: &OrgScope) -> Result<Vec<Campaign>, DbError> {
        Ok(sqlx::query_as::<_, Campaign>(concat!(
            "SELECT ",
            cols!(),
            " FROM campaigns WHERE organization_id = $1 ORDER BY created_at DESC"
        ))
        .bind(scope.organization_id())
        .fetch_all(pool)
        .await?)
    }

    pub async fn get(pool: &PgPool, scope: &OrgScope, id: Uuid) -> Result<Campaign, DbError> {
        sqlx::query_as::<_, Campaign>(concat!(
            "SELECT ",
            cols!(),
            " FROM campaigns WHERE id = $1 AND organization_id = $2"
        ))
        .bind(id)
        .bind(scope.organization_id())
        .fetch_one(pool)
        .await
        .map_err(DbError::from_fetch)
    }

    /// Scalar rollup over the campaign's non-archived backlinks (§22).
    pub async fn rollup(
        pool: &PgPool,
        scope: &OrgScope,
        campaign_id: Uuid,
    ) -> Result<RollupCounts, DbError> {
        Ok(
            sqlx::query_as::<_, RollupCounts>(super::rollup_sql("campaign_id"))
                .bind(scope.organization_id())
                .bind(campaign_id)
                .fetch_one(pool)
                .await?,
        )
    }

    /// Recorded spend for the campaign's backlinks, grouped by currency and
    /// never summed across (§34). `(currency, sum-as-text)`.
    pub async fn spend(
        pool: &PgPool,
        scope: &OrgScope,
        campaign_id: Uuid,
    ) -> Result<Vec<(String, String)>, DbError> {
        Ok(sqlx::query_as::<_, (String, String)>(
            "SELECT currency, sum(cost)::text FROM backlinks \
             WHERE organization_id = $1 AND archived_at IS NULL AND campaign_id = $2 \
               AND cost IS NOT NULL GROUP BY currency",
        )
        .bind(scope.organization_id())
        .bind(campaign_id)
        .fetch_all(pool)
        .await?)
    }
}

// ─── Publishers (§31) — tenant-scoped, with lifespan/removal-rate rollups ─────
pub mod publishers {
    use super::*;
    use crate::entities::{Publisher, RollupCounts};

    macro_rules! cols {
        () => {
            "id, organization_id, name, domain, contact_name, \
             contact_email, notes, created_at, updated_at"
        };
    }

    pub struct NewPublisher<'a> {
        pub name: &'a str,
        pub domain: &'a str,
        pub contact_name: Option<&'a str>,
        pub contact_email: Option<&'a str>,
    }

    pub async fn create(
        pool: &PgPool,
        scope: &OrgScope,
        id: Uuid,
        new: NewPublisher<'_>,
    ) -> Result<Publisher, DbError> {
        sqlx::query_as::<_, Publisher>(concat!(
            "INSERT INTO publishers \
                (id, organization_id, name, domain, contact_name, contact_email) \
             VALUES ($1,$2,$3,$4,$5,$6) \
             RETURNING ",
            cols!()
        ))
        .bind(id)
        .bind(scope.organization_id())
        .bind(new.name)
        .bind(new.domain)
        .bind(new.contact_name)
        .bind(new.contact_email)
        .fetch_one(pool)
        .await
        .map_err(DbError::from_insert)
    }

    pub async fn list(pool: &PgPool, scope: &OrgScope) -> Result<Vec<Publisher>, DbError> {
        Ok(sqlx::query_as::<_, Publisher>(concat!(
            "SELECT ",
            cols!(),
            " FROM publishers WHERE organization_id = $1 ORDER BY name"
        ))
        .bind(scope.organization_id())
        .fetch_all(pool)
        .await?)
    }

    pub async fn get(pool: &PgPool, scope: &OrgScope, id: Uuid) -> Result<Publisher, DbError> {
        sqlx::query_as::<_, Publisher>(concat!(
            "SELECT ",
            cols!(),
            " FROM publishers WHERE id = $1 AND organization_id = $2"
        ))
        .bind(id)
        .bind(scope.organization_id())
        .fetch_one(pool)
        .await
        .map_err(DbError::from_fetch)
    }

    /// Scalar rollup over the publisher's non-archived backlinks (§31):
    /// live/lost counts plus average lifespan of lost links. Removal rate is
    /// `lost / total`, derived by the caller so it need not be recomputed here.
    pub async fn rollup(
        pool: &PgPool,
        scope: &OrgScope,
        publisher_id: Uuid,
    ) -> Result<RollupCounts, DbError> {
        Ok(
            sqlx::query_as::<_, RollupCounts>(super::rollup_sql("publisher_id"))
                .bind(scope.organization_id())
                .bind(publisher_id)
                .fetch_one(pool)
                .await?,
        )
    }

    /// Recorded spend for the publisher's backlinks, grouped by currency (§34).
    pub async fn spend(
        pool: &PgPool,
        scope: &OrgScope,
        publisher_id: Uuid,
    ) -> Result<Vec<(String, String)>, DbError> {
        Ok(sqlx::query_as::<_, (String, String)>(
            "SELECT currency, sum(cost)::text FROM backlinks \
             WHERE organization_id = $1 AND archived_at IS NULL AND publisher_id = $2 \
               AND cost IS NOT NULL GROUP BY currency",
        )
        .bind(scope.organization_id())
        .bind(publisher_id)
        .fetch_all(pool)
        .await?)
    }
}

/// The shared rollup aggregate, selected by the grouping FK column. The column
/// name is a fixed `&'static str` (`"campaign_id"`/`"publisher_id"`), never user
/// input, so the resulting SQL is still an audited literal (S9). `$1` =
/// organization_id, `$2` = the fk id.
fn rollup_sql(fk: &'static str) -> &'static str {
    match fk {
        "campaign_id" => ROLLUP_CAMPAIGN,
        "publisher_id" => ROLLUP_PUBLISHER,
        _ => unreachable!("rollup_sql called with a non-literal fk"),
    }
}

macro_rules! rollup_query {
    ($fk:literal) => {
        concat!(
            "SELECT \
                count(*)::bigint AS total, \
                count(*) FILTER (WHERE status = 'live')::bigint AS live, \
                count(*) FILTER (WHERE status = 'lost')::bigint AS lost, \
                count(*) FILTER (WHERE status IN \
                    ('changed','nofollow','redirected','source_error','target_error','blocked') \
                )::bigint AS attention, \
                avg((EXTRACT(EPOCH FROM (last_seen_at - created_at)) / 86400.0)::float8) \
                    FILTER (WHERE status = 'lost' AND last_seen_at IS NOT NULL) \
                    AS avg_lifespan_days \
             FROM backlinks \
             WHERE organization_id = $1 AND archived_at IS NULL AND ",
            $fk,
            " = $2"
        )
    };
}

const ROLLUP_CAMPAIGN: &str = rollup_query!("campaign_id");
const ROLLUP_PUBLISHER: &str = rollup_query!("publisher_id");

// ─── Warranties (§32) — the system-wide daily expiry sweep ───────────────────
pub mod warranties {
    use super::*;

    /// One warranted link that is at or past its expiry lookahead. `org_id`
    /// travels with the row so the sweep can write an org-scoped notification
    /// without a second query.
    #[derive(Debug, Clone, sqlx::FromRow)]
    pub struct ExpiringWarranty {
        pub organization_id: Uuid,
        pub backlink_id: Uuid,
        pub warranty_expiration: chrono::NaiveDate,
    }

    /// Every org's warranted links whose warranty lapses on or before `cutoff`.
    /// This is a **system** query (the daily `check_warranties` job, docs/07 §5),
    /// not tenant-scoped — each returned row still carries its `organization_id`
    /// so downstream writes stay org-scoped. Bounded by `limit` for batching.
    pub async fn expiring_before(
        pool: &PgPool,
        cutoff: chrono::NaiveDate,
        limit: i64,
    ) -> Result<Vec<ExpiringWarranty>, DbError> {
        Ok(sqlx::query_as::<_, ExpiringWarranty>(
            "SELECT organization_id, id AS backlink_id, warranty_expiration \
             FROM backlinks \
             WHERE archived_at IS NULL \
               AND warranty_expiration IS NOT NULL AND warranty_expiration <= $1 \
             ORDER BY warranty_expiration \
             LIMIT $2",
        )
        .bind(cutoff)
        .bind(limit.clamp(1, 5000))
        .fetch_all(pool)
        .await?)
    }
}

// ─── Notifications (§37) — write side used by the warranty sweep ──────────────
pub mod notifications {
    use super::*;

    /// Record a warranty-expiry alert, idempotently. `dedupe_key` keys on the
    /// backlink and its expiration date, so a daily re-run does not notify twice
    /// for the same lapse, but a re-warrantied link (new expiration) can alert
    /// again (docs/07 §4). Returns true when a new row was written.
    pub async fn create_warranty_alert(
        pool: &PgPool,
        organization_id: Uuid,
        backlink_id: Uuid,
        expiration: chrono::NaiveDate,
        title: &str,
        body: &str,
    ) -> Result<bool, DbError> {
        let dedupe_key = format!("warranty:{backlink_id}:{expiration}");
        let row: Option<(Uuid,)> = sqlx::query_as(
            "INSERT INTO notifications \
                (id, organization_id, backlink_id, kind, title, body, dedupe_key) \
             VALUES ($1, $2, $3, 'warranty_expiring', $4, $5, $6) \
             ON CONFLICT (dedupe_key) WHERE dedupe_key IS NOT NULL DO NOTHING \
             RETURNING id",
        )
        .bind(Uuid::now_v7())
        .bind(organization_id)
        .bind(backlink_id)
        .bind(title)
        .bind(body)
        .bind(&dedupe_key)
        .fetch_optional(pool)
        .await?;
        Ok(row.is_some())
    }

    /// Count of unread notifications for an org — the alerts badge.
    pub async fn unread_count(pool: &PgPool, scope: &OrgScope) -> Result<i64, DbError> {
        let (n,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM notifications \
             WHERE organization_id = $1 AND read_at IS NULL",
        )
        .bind(scope.organization_id())
        .fetch_one(pool)
        .await?;
        Ok(n)
    }
}
