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
             created_at, updated_at, next_check_at, last_checked_at"
        };
    }

    /// Input for creating a backlink. URLs are normalized here (docs/03 §3.4)
    /// via the same normalizer the crawler uses, so a stored link and a
    /// discovered link compare identically.
    pub struct NewBacklink<'a> {
        pub project_id: Uuid,
        pub source_url: &'a str,
        pub target_url: &'a str,
        pub expected_anchor_text: Option<&'a str>,
        pub expected_rel_type: &'a str,
        pub cost: Option<&'a str>,
        pub currency: Option<&'a str>,
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

        sqlx::query_as::<_, Backlink>(concat!(
            "INSERT INTO backlinks (id, organization_id, project_id, \
                source_url, source_url_normalized, source_domain, \
                target_url, target_url_normalized, target_domain, \
                expected_anchor_text, expected_rel_type, cost, currency) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11::rel_type,$12::numeric,$13) \
             RETURNING ",
            cols!()
        ))
        .bind(id)
        .bind(scope.organization_id())
        .bind(new.project_id)
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
