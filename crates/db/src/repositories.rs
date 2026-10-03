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

/// Optional provider-supplied metadata for a discovered backlink (e.g. DataForSEO
/// first_seen / last_seen / page HTTP status), so a discovered row is populated
/// before our own first crawl. None for sources that don't supply it.
#[derive(Debug, Clone, Default)]
pub struct DiscoveredMeta {
    pub first_seen: Option<DateTime<Utc>>,
    pub last_seen: Option<DateTime<Utc>>,
    pub source_status: Option<i32>,
}

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
             RETURNING id, email, password_hash, name, email_verified_at, \
                       is_platform_admin, created_at, updated_at",
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
            "SELECT id, email, password_hash, name, email_verified_at, \
                    is_platform_admin, created_at, updated_at \
             FROM users WHERE email = $1",
        )
        .bind(email)
        .fetch_optional(pool)
        .await?)
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<User, DbError> {
        sqlx::query_as::<_, User>(
            "SELECT id, email, password_hash, name, email_verified_at, \
                    is_platform_admin, created_at, updated_at \
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

    /// Replace a user's password hash (password reset).
    pub async fn set_password(pool: &PgPool, id: Uuid, password_hash: &str) -> Result<(), DbError> {
        sqlx::query("UPDATE users SET password_hash = $2, updated_at = now() WHERE id = $1")
            .bind(id)
            .bind(password_hash)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Grant or revoke the platform-admin flag (backoffice; migration 0022).
    pub async fn set_platform_admin(pool: &PgPool, id: Uuid, on: bool) -> Result<User, DbError> {
        sqlx::query_as::<_, User>(
            "UPDATE users SET is_platform_admin = $2, updated_at = now() WHERE id = $1 \
             RETURNING id, email, password_hash, name, email_verified_at, \
                       is_platform_admin, created_at, updated_at",
        )
        .bind(id)
        .bind(on)
        .fetch_one(pool)
        .await
        .map_err(DbError::from_fetch)
    }

    /// Bootstrap: ensure the accounts in `emails` (case-insensitive) are platform
    /// admins. Idempotent — only flips rows that aren't already set. Returns how
    /// many were newly promoted. Called at API boot from the ADMIN_EMAILS env, so
    /// no credentials are committed: the operator registers, then is promoted.
    pub async fn promote_admins_by_email(pool: &PgPool, emails: &[String]) -> Result<u64, DbError> {
        if emails.is_empty() {
            return Ok(0);
        }
        let lowered: Vec<String> = emails.iter().map(|e| e.to_lowercase()).collect();
        let res = sqlx::query(
            "UPDATE users SET is_platform_admin = true, updated_at = now() \
             WHERE lower(email) = ANY($1) AND is_platform_admin = false",
        )
        .bind(&lowered)
        .execute(pool)
        .await?;
        Ok(res.rows_affected())
    }
}

// ─── Auth tokens: single-use email-verify / password-reset (table from 0002) ──
// Purpose values are 'email_verification' | 'password_reset' (0002 CHECK).
pub mod auth_tokens {
    use super::*;

    pub async fn create(
        pool: &PgPool,
        id: Uuid,
        user_id: Uuid,
        purpose: &str,
        token_hash: &[u8],
        expires_at: DateTime<Utc>,
    ) -> Result<(), DbError> {
        sqlx::query(
            "INSERT INTO auth_tokens (id, user_id, purpose, token_hash, expires_at) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(id)
        .bind(user_id)
        .bind(purpose)
        .bind(token_hash)
        .bind(expires_at)
        .execute(pool)
        .await
        .map_err(DbError::from_insert)?;
        Ok(())
    }

    /// Atomically spend a token: mark it consumed and return its user id, only if
    /// it matches the purpose, is unconsumed and unexpired. `None` otherwise.
    pub async fn consume(
        pool: &PgPool,
        purpose: &str,
        token_hash: &[u8],
    ) -> Result<Option<Uuid>, DbError> {
        let row: Option<(Uuid,)> = sqlx::query_as(
            "UPDATE auth_tokens SET consumed_at = now() \
             WHERE token_hash = $1 AND purpose = $2 AND consumed_at IS NULL AND expires_at > now() \
             RETURNING user_id",
        )
        .bind(token_hash)
        .bind(purpose)
        .fetch_optional(pool)
        .await?;
        Ok(row.map(|(u,)| u))
    }
}

// ─── Platform admin backoffice (cross-tenant; migration 0022) ─────────────────
//
// The ONLY sanctioned cross-tenant read surface. Every caller is gated on
// `users.is_platform_admin` in the route layer before any of these run. Reads are
// capped so a large instance can't return an unbounded payload.
pub mod admin {
    use super::*;

    #[derive(Debug, Clone, sqlx::FromRow)]
    pub struct PlatformStats {
        pub users: i64,
        pub organizations: i64,
        pub projects: i64,
        pub backlinks: i64,
        pub backlinks_live: i64,
        pub backlinks_lost: i64,
    }

    pub async fn platform_stats(pool: &PgPool) -> Result<PlatformStats, DbError> {
        sqlx::query_as::<_, PlatformStats>(
            "SELECT \
               (SELECT count(*) FROM users)::bigint AS users, \
               (SELECT count(*) FROM organizations)::bigint AS organizations, \
               (SELECT count(*) FROM projects)::bigint AS projects, \
               (SELECT count(*) FROM backlinks)::bigint AS backlinks, \
               (SELECT count(*) FROM backlinks WHERE status = 'live')::bigint AS backlinks_live, \
               (SELECT count(*) FROM backlinks WHERE status = 'lost')::bigint AS backlinks_lost",
        )
        .fetch_one(pool)
        .await
        .map_err(DbError::from_fetch)
    }

    #[derive(Debug, Clone, sqlx::FromRow)]
    pub struct AdminOrg {
        pub id: Uuid,
        pub name: String,
        pub slug: String,
        pub created_at: DateTime<Utc>,
        pub suspended: bool,
        pub plan: String,
        pub members: i64,
        pub projects: i64,
        pub backlinks: i64,
        pub owner_email: Option<String>,
    }

    pub async fn list_organizations(pool: &PgPool) -> Result<Vec<AdminOrg>, DbError> {
        Ok(sqlx::query_as::<_, AdminOrg>(
            "SELECT o.id, o.name, o.slug, o.created_at, \
               (o.suspended_at IS NOT NULL) AS suspended, \
               COALESCE((SELECT s.plan::text FROM subscriptions s \
                  WHERE s.organization_id = o.id \
                    AND s.status IN ('trialing','active','past_due') LIMIT 1), 'starter') AS plan, \
               (SELECT count(*) FROM organization_members m \
                  WHERE m.organization_id = o.id)::bigint AS members, \
               (SELECT count(*) FROM projects p \
                  WHERE p.organization_id = o.id)::bigint AS projects, \
               (SELECT count(*) FROM backlinks b \
                  WHERE b.organization_id = o.id)::bigint AS backlinks, \
               (SELECT u.email FROM organization_members m \
                  JOIN users u ON u.id = m.user_id \
                  WHERE m.organization_id = o.id AND m.role = 'owner' \
                  ORDER BY u.created_at LIMIT 1) AS owner_email \
             FROM organizations o \
             ORDER BY o.created_at DESC \
             LIMIT 500",
        )
        .fetch_all(pool)
        .await?)
    }

    /// Suspend or unsuspend an org (migration 0023). Returns the org name for the
    /// audit trail. Idempotent.
    pub async fn set_suspended(pool: &PgPool, org_id: Uuid, on: bool) -> Result<String, DbError> {
        let row: Option<(String,)> = sqlx::query_as(
            "UPDATE organizations \
             SET suspended_at = CASE WHEN $2 THEN COALESCE(suspended_at, now()) ELSE NULL END, \
                 updated_at = now() \
             WHERE id = $1 RETURNING name",
        )
        .bind(org_id)
        .bind(on)
        .fetch_optional(pool)
        .await?;
        row.map(|(n,)| n).ok_or(DbError::NotFound)
    }

    /// Hard-delete an org and everything it owns (FK cascades). Returns the name
    /// for the audit trail, or NotFound. Destructive and irreversible.
    pub async fn delete_organization(pool: &PgPool, org_id: Uuid) -> Result<String, DbError> {
        let row: Option<(String,)> =
            sqlx::query_as("DELETE FROM organizations WHERE id = $1 RETURNING name")
                .bind(org_id)
                .fetch_optional(pool)
                .await?;
        row.map(|(n,)| n).ok_or(DbError::NotFound)
    }

    /// Admin plan override: upsert an `active` subscription so `plan_for_org`
    /// resolves to `plan_slug`. Keeps any existing Stripe customer id on update;
    /// only sets a placeholder on first insert (manual/comped plan, no Stripe).
    pub async fn set_plan(pool: &PgPool, org_id: Uuid, plan_slug: &str) -> Result<(), DbError> {
        sqlx::query(
            "INSERT INTO subscriptions (id, organization_id, stripe_customer_id, plan, status) \
             VALUES ($1, $2, $3, $4::subscription_plan, 'active') \
             ON CONFLICT (organization_id) DO UPDATE \
               SET plan = EXCLUDED.plan, status = 'active', updated_at = now()",
        )
        .bind(Uuid::now_v7())
        .bind(org_id)
        .bind(format!("admin:{org_id}"))
        .bind(plan_slug)
        .execute(pool)
        .await
        .map_err(DbError::from_insert)?;
        Ok(())
    }

    #[derive(Debug, Clone, sqlx::FromRow)]
    pub struct AdminUser {
        pub id: Uuid,
        pub email: String,
        pub name: String,
        pub is_platform_admin: bool,
        pub email_verified_at: Option<DateTime<Utc>>,
        pub created_at: DateTime<Utc>,
        pub organizations: i64,
    }

    pub async fn list_users(pool: &PgPool) -> Result<Vec<AdminUser>, DbError> {
        Ok(sqlx::query_as::<_, AdminUser>(
            "SELECT u.id, u.email, u.name, u.is_platform_admin, u.email_verified_at, \
               u.created_at, \
               (SELECT count(*) FROM organization_members m \
                  WHERE m.user_id = u.id)::bigint AS organizations \
             FROM users u \
             ORDER BY u.created_at DESC \
             LIMIT 500",
        )
        .fetch_all(pool)
        .await?)
    }

    // ─── Founder-ops read models (docs/19) ───────────────────────────────────
    // All derived from tables that already exist — no new instrumentation. The
    // windowed counts take seconds so the caller controls the look-back.

    #[derive(Debug, Clone, sqlx::FromRow)]
    pub struct OpsHealth {
        pub q_pending: i64,
        pub q_processing: i64,
        pub q_failed: i64,
        pub q_dead: i64,
        pub q_oldest_pending_at: Option<DateTime<Utc>>,
        pub q_last_completed_at: Option<DateTime<Utc>>,
        pub q_created_1h: i64,
        pub checks: i64,
        pub checks_failed: i64,
        pub avg_duration_ms: Option<f64>,
        pub lost_detected: i64,
        pub email_sent: i64,
        pub email_failed: i64,
    }

    /// Queue (jobs), monitoring (backlink_checks over `window_secs`) and
    /// alert-email (alert_deliveries) health in one snapshot.
    pub async fn ops_health(pool: &PgPool, window_secs: f64) -> Result<OpsHealth, DbError> {
        sqlx::query_as::<_, OpsHealth>(
            "SELECT \
               (SELECT count(*) FROM jobs WHERE status = 'pending')::bigint AS q_pending, \
               (SELECT count(*) FROM jobs WHERE status = 'processing')::bigint AS q_processing, \
               (SELECT count(*) FROM jobs WHERE status = 'failed')::bigint AS q_failed, \
               (SELECT count(*) FROM jobs WHERE status = 'dead')::bigint AS q_dead, \
               (SELECT min(scheduled_at) FROM jobs WHERE status = 'pending') AS q_oldest_pending_at, \
               (SELECT max(completed_at) FROM jobs WHERE status = 'completed') AS q_last_completed_at, \
               (SELECT count(*) FROM jobs WHERE created_at > now() - interval '1 hour')::bigint AS q_created_1h, \
               (SELECT count(*) FROM backlink_checks WHERE checked_at > now() - interval '1 second' * $1::double precision)::bigint AS checks, \
               (SELECT count(*) FROM backlink_checks WHERE checked_at > now() - interval '1 second' * $1::double precision AND (found = false OR error_kind IS NOT NULL))::bigint AS checks_failed, \
               (SELECT AVG(duration_ms)::double precision FROM backlink_checks WHERE checked_at > now() - interval '1 second' * $1::double precision) AS avg_duration_ms, \
               (SELECT count(*) FROM backlink_checks WHERE checked_at > now() - interval '1 second' * $1::double precision AND status_after = 'lost')::bigint AS lost_detected, \
               (SELECT count(*) FROM alert_deliveries d JOIN alert_destinations a ON a.id = d.destination_id WHERE a.kind = 'email' AND d.created_at > now() - interval '1 second' * $1::double precision)::bigint AS email_sent, \
               (SELECT count(*) FROM alert_deliveries d JOIN alert_destinations a ON a.id = d.destination_id WHERE a.kind = 'email' AND d.status = 'error' AND d.created_at > now() - interval '1 second' * $1::double precision)::bigint AS email_failed",
        )
        .bind(window_secs)
        .fetch_one(pool)
        .await
        .map_err(DbError::from_fetch)
    }

    #[derive(Debug, Clone, sqlx::FromRow)]
    pub struct BillingCounts {
        pub active: i64,
        pub trialing: i64,
        /// Trials still live (`trial_end` in the future).
        pub trialing_active: i64,
        /// Trials whose window has elapsed but never converted (access locked).
        pub trialing_expired: i64,
        pub past_due: i64,
        pub canceled: i64,
        pub comped: i64,
    }

    /// Subscription status counts, including the trial funnel split. `comped` =
    /// active/past_due rows with the admin placeholder customer id, so the handler
    /// can keep them out of MRR.
    pub async fn billing_counts(pool: &PgPool) -> Result<BillingCounts, DbError> {
        sqlx::query_as::<_, BillingCounts>(
            "SELECT \
               (SELECT count(*) FROM subscriptions WHERE status = 'active')::bigint AS active, \
               (SELECT count(*) FROM subscriptions WHERE status = 'trialing')::bigint AS trialing, \
               (SELECT count(*) FROM subscriptions \
                  WHERE status = 'trialing' AND trial_end > now())::bigint AS trialing_active, \
               (SELECT count(*) FROM subscriptions \
                  WHERE status = 'trialing' \
                    AND (trial_end IS NULL OR trial_end <= now()))::bigint AS trialing_expired, \
               (SELECT count(*) FROM subscriptions WHERE status = 'past_due')::bigint AS past_due, \
               (SELECT count(*) FROM subscriptions WHERE status = 'canceled')::bigint AS canceled, \
               (SELECT count(*) FROM subscriptions WHERE status IN ('active','past_due') \
                  AND stripe_customer_id LIKE 'admin:%')::bigint AS comped",
        )
        .fetch_one(pool)
        .await
        .map_err(DbError::from_fetch)
    }

    #[derive(Debug, Clone, sqlx::FromRow)]
    pub struct AdminSubscription {
        pub id: Uuid,
        pub name: String,
        pub owner_email: Option<String>,
        pub plan: String,
        pub status: String,
        pub trial_end: Option<DateTime<Utc>>,
        pub current_period_end: Option<DateTime<Utc>>,
        pub cancel_at_period_end: bool,
        pub canceled_at: Option<DateTime<Utc>>,
        /// Admin-set (comped) subscription — excluded from revenue.
        pub comped: bool,
    }

    /// Every org that has a subscription row, with its billing state (plan,
    /// status, trial end, renewal date, pending cancel). Problem/active states
    /// first; the handler prices each against the catalog for per-customer MRR.
    pub async fn list_subscriptions(pool: &PgPool) -> Result<Vec<AdminSubscription>, DbError> {
        Ok(sqlx::query_as::<_, AdminSubscription>(
            "SELECT o.id, o.name, \
               (SELECT u.email FROM organization_members m \
                  JOIN users u ON u.id = m.user_id \
                  WHERE m.organization_id = o.id AND m.role = 'owner' \
                  ORDER BY u.created_at LIMIT 1) AS owner_email, \
               s.plan::text AS plan, s.status, s.trial_end, s.current_period_end, \
               s.cancel_at_period_end, s.canceled_at, \
               (s.stripe_customer_id LIKE 'admin:%') AS comped \
             FROM subscriptions s \
             JOIN organizations o ON o.id = s.organization_id \
             ORDER BY \
               CASE s.status \
                 WHEN 'past_due' THEN 0 WHEN 'active' THEN 1 \
                 WHEN 'trialing' THEN 2 WHEN 'canceled' THEN 4 ELSE 3 END, \
               s.current_period_end ASC NULLS LAST, s.trial_end ASC NULLS LAST \
             LIMIT 500",
        )
        .fetch_all(pool)
        .await?)
    }

    #[derive(Debug, Clone, sqlx::FromRow)]
    pub struct PlanCount {
        pub plan: String,
        pub n: i64,
    }

    /// Real paying subscriptions per plan (excludes admin-comped rows), so the
    /// handler can price them against the plan catalog for MRR.
    pub async fn paying_by_plan(pool: &PgPool) -> Result<Vec<PlanCount>, DbError> {
        Ok(sqlx::query_as::<_, PlanCount>(
            "SELECT plan::text AS plan, count(*)::bigint AS n \
             FROM subscriptions \
             WHERE status IN ('active','past_due') AND stripe_customer_id NOT LIKE 'admin:%' \
             GROUP BY plan",
        )
        .fetch_all(pool)
        .await?)
    }

    #[derive(Debug, Clone, sqlx::FromRow)]
    pub struct UsageRow {
        pub id: Uuid,
        pub name: String,
        pub plan: String,
        pub backlinks: i64,
    }

    /// Active workspaces with their plan + monitored-backlink count, busiest
    /// first. The handler joins this to the plan-limit catalog to find who is
    /// near their cap.
    pub async fn usage_vs_limit(pool: &PgPool) -> Result<Vec<UsageRow>, DbError> {
        Ok(sqlx::query_as::<_, UsageRow>(
            "SELECT o.id, o.name, \
               COALESCE((SELECT s.plan::text FROM subscriptions s \
                  WHERE s.organization_id = o.id \
                    AND s.status IN ('trialing','active','past_due') LIMIT 1), 'starter') AS plan, \
               (SELECT count(*) FROM backlinks b WHERE b.organization_id = o.id)::bigint AS backlinks \
             FROM organizations o \
             WHERE o.suspended_at IS NULL \
             ORDER BY backlinks DESC \
             LIMIT 50",
        )
        .fetch_all(pool)
        .await?)
    }
}

// ─── Audit log (docs/03 §3.8, §45) — append-only trail of privileged actions ───
pub mod audit {
    use super::*;
    use serde_json::Value;

    /// Append one audit entry. Best-effort at the call site: a failure to log must
    /// not fail the action it records, so callers typically `let _ =` the result.
    /// `org_id` is None for actions whose org is being deleted (so the row is not
    /// cascade-removed with it — the org id then lives in `resource_id`).
    #[allow(clippy::too_many_arguments)]
    pub async fn log(
        pool: &PgPool,
        org_id: Option<Uuid>,
        actor_user_id: Option<Uuid>,
        actor_email: Option<&str>,
        action: &str,
        resource_type: Option<&str>,
        resource_id: Option<Uuid>,
        before: Option<Value>,
        after: Option<Value>,
    ) -> Result<(), DbError> {
        sqlx::query(
            "INSERT INTO audit_logs \
               (id, organization_id, actor_user_id, actor_email, action, \
                resource_type, resource_id, before, after) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(Uuid::now_v7())
        .bind(org_id)
        .bind(actor_user_id)
        .bind(actor_email)
        .bind(action)
        .bind(resource_type)
        .bind(resource_id)
        .bind(before)
        .bind(after)
        .execute(pool)
        .await?;
        Ok(())
    }

    #[derive(Debug, Clone, sqlx::FromRow)]
    pub struct AuditEntry {
        pub id: Uuid,
        pub created_at: DateTime<Utc>,
        pub organization_id: Option<Uuid>,
        pub org_name: Option<String>,
        pub actor_email: Option<String>,
        pub action: String,
        pub resource_type: Option<String>,
        pub resource_id: Option<Uuid>,
        pub after: Option<String>,
    }

    /// Cross-tenant recent audit list for the platform backoffice (admin-gated at
    /// the route layer). `after` is rendered as text for display.
    pub async fn list_recent(pool: &PgPool, limit: i64) -> Result<Vec<AuditEntry>, DbError> {
        Ok(sqlx::query_as::<_, AuditEntry>(
            "SELECT a.id, a.created_at, a.organization_id, o.name AS org_name, \
               a.actor_email::text AS actor_email, a.action, a.resource_type, \
               a.resource_id, a.after::text AS after \
             FROM audit_logs a \
             LEFT JOIN organizations o ON o.id = a.organization_id \
             ORDER BY a.created_at DESC \
             LIMIT $1",
        )
        .bind(limit.clamp(1, 500))
        .fetch_all(pool)
        .await?)
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

    /// Optional contact/company details for a client (§20).
    #[derive(Debug, Default)]
    pub struct NewClient<'a> {
        pub company_name: Option<&'a str>,
        pub website: Option<&'a str>,
        pub contact_name: Option<&'a str>,
        pub contact_email: Option<&'a str>,
        pub notes: Option<&'a str>,
    }

    pub async fn create(
        pool: &PgPool,
        scope: &OrgScope,
        id: Uuid,
        name: &str,
        extra: NewClient<'_>,
    ) -> Result<Client, DbError> {
        sqlx::query_as::<_, Client>(
            "INSERT INTO clients \
                (id, organization_id, name, company_name, website, contact_name, contact_email, notes) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
             RETURNING id, organization_id, name, company_name, website, \
                       contact_name, contact_email, notes, created_at, updated_at",
        )
        .bind(id)
        .bind(scope.organization_id())
        .bind(name)
        .bind(extra.company_name)
        .bind(extra.website)
        .bind(extra.contact_name)
        .bind(extra.contact_email)
        .bind(extra.notes)
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

    /// Delete a client (§20). Its projects are unlinked first (the composite
    /// `client_id` FK's plain SET NULL would also null the NOT NULL
    /// `organization_id`). `NotFound` if not in the org.
    pub async fn delete(pool: &PgPool, scope: &OrgScope, id: Uuid) -> Result<(), DbError> {
        let mut tx = pool.begin().await?;
        sqlx::query(
            "UPDATE projects SET client_id = NULL \
             WHERE client_id = $1 AND organization_id = $2",
        )
        .bind(id)
        .bind(scope.organization_id())
        .execute(&mut *tx)
        .await?;
        let row: Option<(Uuid,)> = sqlx::query_as(
            "DELETE FROM clients WHERE id = $1 AND organization_id = $2 RETURNING id",
        )
        .bind(id)
        .bind(scope.organization_id())
        .fetch_optional(&mut *tx)
        .await?;
        if row.is_none() {
            return Err(DbError::NotFound);
        }
        tx.commit().await?;
        Ok(())
    }

    /// Full-form edit of a client (§20): name plus the optional contact/company
    /// fields (a `None`/empty field clears it). `NotFound` if not in the org.
    pub async fn update(
        pool: &PgPool,
        scope: &OrgScope,
        id: Uuid,
        name: &str,
        extra: NewClient<'_>,
    ) -> Result<Client, DbError> {
        sqlx::query_as::<_, Client>(
            "UPDATE clients SET name = $3, company_name = $4, website = $5, \
                    contact_name = $6, contact_email = $7, notes = $8, updated_at = now() \
             WHERE id = $1 AND organization_id = $2 \
             RETURNING id, organization_id, name, company_name, website, \
                       contact_name, contact_email, notes, created_at, updated_at",
        )
        .bind(id)
        .bind(scope.organization_id())
        .bind(name)
        .bind(extra.company_name)
        .bind(extra.website)
        .bind(extra.contact_name)
        .bind(extra.contact_email)
        .bind(extra.notes)
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
        client_id: Option<Uuid>,
    ) -> Result<Project, DbError> {
        sqlx::query_as::<_, Project>(
            "WITH inserted AS ( \
                INSERT INTO projects \
                    (id, organization_id, client_id, name, domain, default_check_frequency) \
                VALUES ($1, $2, $3, $4, $5, make_interval(secs => $6)) \
                RETURNING * \
             ) \
             SELECT id, organization_id, client_id, name, domain, \
                    EXTRACT(EPOCH FROM default_check_frequency)::bigint AS check_frequency_secs, \
                    timezone, status::text AS status, created_at, updated_at, archived_at \
             FROM inserted",
        )
        .bind(id)
        .bind(scope.organization_id())
        .bind(client_id)
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

    /// Active (non-archived) project count for the org — for plan-limit checks.
    pub async fn count(pool: &PgPool, scope: &OrgScope) -> Result<i64, DbError> {
        let (n,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM projects WHERE organization_id = $1 AND archived_at IS NULL",
        )
        .bind(scope.organization_id())
        .fetch_one(pool)
        .await?;
        Ok(n)
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

    /// Partial update (§21): rename, change the check frequency, and/or
    /// reassign the client. Each argument is "apply if `Some`, keep if `None`";
    /// `client` is a double option so `Some(None)` clears the client. Built with
    /// `QueryBuilder` so only the provided columns are written (bound params, so
    /// still injection-safe — S9). The caller guarantees at least one field.
    /// `NotFound` if the project is not in the org.
    pub async fn update(
        pool: &PgPool,
        scope: &OrgScope,
        id: Uuid,
        name: Option<&str>,
        check_frequency_secs: Option<i64>,
        client: Option<Option<Uuid>>,
    ) -> Result<Project, DbError> {
        let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new("UPDATE projects SET ");
        let mut sep = qb.separated(", ");
        if let Some(n) = name {
            sep.push("name = ");
            sep.push_bind_unseparated(n);
        }
        if let Some(s) = check_frequency_secs {
            sep.push("default_check_frequency = make_interval(secs => ");
            sep.push_bind_unseparated(s as f64);
            sep.push_unseparated(")");
        }
        if let Some(c) = client {
            sep.push("client_id = ");
            sep.push_bind_unseparated(c); // Option<Uuid> binds NULL to clear
        }
        qb.push(" WHERE id = ");
        qb.push_bind(id);
        qb.push(" AND organization_id = ");
        qb.push_bind(scope.organization_id());
        qb.push(
            " RETURNING id, organization_id, client_id, name, domain, \
              EXTRACT(EPOCH FROM default_check_frequency)::bigint AS check_frequency_secs, \
              timezone, status::text AS status, created_at, updated_at, archived_at",
        );
        qb.build_query_as::<Project>()
            .fetch_one(pool)
            .await
            .map_err(DbError::from_fetch)
    }

    /// Delete a project and everything under it — backlinks, campaigns, imports,
    /// competitors, discovery seeds all cascade via their composite FK
    /// (`… REFERENCES projects (organization_id, id) ON DELETE CASCADE`).
    /// Irreversible. `NotFound` if not in the org.
    pub async fn delete(pool: &PgPool, scope: &OrgScope, id: Uuid) -> Result<(), DbError> {
        let row: Option<(Uuid,)> = sqlx::query_as(
            "DELETE FROM projects WHERE id = $1 AND organization_id = $2 RETURNING id",
        )
        .bind(id)
        .bind(scope.organization_id())
        .fetch_optional(pool)
        .await?;
        if row.is_none() {
            return Err(DbError::NotFound);
        }
        Ok(())
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
             created_at, updated_at, next_check_at, last_checked_at, \
             detected_anchor_text, detected_target_url, detected_rel_attributes, \
             detected_is_dofollow, source_http_status, target_http_status, \
             first_checked_at, last_seen_at, link_type, image_alt, lost_at, target_final_url, \
             subject, competitor_id, tier, parent_backlink_id"
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

    /// The mutable fields of a backlink (§36 edit). Source/target URLs are not
    /// editable — they define the link's identity (change = delete + recreate).
    /// Every field is the FULL new value, so `None` clears a nullable column.
    pub struct BacklinkUpdate<'a> {
        pub expected_anchor_text: Option<&'a str>,
        pub expected_rel_type: &'a str,
        pub cost: Option<&'a str>,
        pub currency: Option<&'a str>,
        pub campaign_id: Option<Uuid>,
        pub publisher_id: Option<Uuid>,
    }

    /// Replace a backlink's mutable fields (§36). Tenant-scoped; `NotFound` if it
    /// is not in the org. The caller merges the PATCH over current values and
    /// validates before calling, so this is a straight write.
    pub async fn update(
        pool: &PgPool,
        scope: &OrgScope,
        id: Uuid,
        upd: BacklinkUpdate<'_>,
    ) -> Result<Backlink, DbError> {
        sqlx::query_as::<_, Backlink>(concat!(
            "UPDATE backlinks SET \
                expected_anchor_text = $3, \
                expected_rel_type = $4::rel_type, \
                cost = $5::numeric, \
                currency = $6, \
                campaign_id = $7, \
                publisher_id = $8 \
             WHERE id = $1 AND organization_id = $2 RETURNING ",
            cols!()
        ))
        .bind(id)
        .bind(scope.organization_id())
        .bind(upd.expected_anchor_text)
        .bind(upd.expected_rel_type)
        .bind(upd.cost)
        .bind(upd.currency)
        .bind(upd.campaign_id)
        .bind(upd.publisher_id)
        .fetch_one(pool)
        .await
        .map_err(DbError::from_fetch)
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

    /// Optional filters for the backlink list (§35).
    #[derive(Debug, Default)]
    pub struct BacklinkFilter<'a> {
        pub project_id: Option<Uuid>,
        /// Filter to backlinks whose project is filed under this client (§20).
        pub client_id: Option<Uuid>,
        /// A `backlink_status` value; the caller validates it against the enum.
        pub status: Option<&'a str>,
        pub tag_id: Option<Uuid>,
        /// Free-text: matches source/target domain or anchor (case-insensitive).
        pub q: Option<&'a str>,
        /// Pagination offset (rows to skip); 0 = first page.
        pub offset: i64,
        /// Acquisition-period cutoff on first-seen; None = all time.
        pub since: Option<DateTime<Utc>>,
    }

    /// Filtered backlink list (§35). Built with sqlx's `QueryBuilder`, so every
    /// value is a bound parameter (never string-interpolated) — the same
    /// injection guarantee as the literal queries, for a dynamic WHERE. Archived
    /// links are always excluded; newest first.
    pub async fn search(
        pool: &PgPool,
        scope: &OrgScope,
        filter: &BacklinkFilter<'_>,
        limit: i64,
    ) -> Result<Vec<Backlink>, DbError> {
        let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new(concat!(
            "SELECT ",
            cols!(),
            " FROM backlinks WHERE organization_id = "
        ));
        qb.push_bind(scope.organization_id());
        qb.push(" AND archived_at IS NULL AND subject = 'own' AND tier = 1");

        if let Some(pid) = filter.project_id {
            qb.push(" AND project_id = ");
            qb.push_bind(pid);
        }
        if let Some(client) = filter.client_id {
            qb.push(" AND project_id IN (SELECT id FROM projects WHERE organization_id = ");
            qb.push_bind(scope.organization_id());
            qb.push(" AND client_id = ");
            qb.push_bind(client);
            qb.push(")");
        }
        if let Some(status) = filter.status {
            qb.push(" AND status = ");
            qb.push_bind(status.to_string());
            qb.push("::backlink_status");
        }
        if let Some(q) = filter.q.map(str::trim).filter(|s| !s.is_empty()) {
            let pat = format!("%{}%", q.replace('%', "\\%").replace('_', "\\_"));
            qb.push(" AND (source_domain ILIKE ");
            qb.push_bind(pat.clone());
            qb.push(" OR target_domain ILIKE ");
            qb.push_bind(pat.clone());
            qb.push(" OR expected_anchor_text ILIKE ");
            qb.push_bind(pat);
            qb.push(")");
        }
        if let Some(tag) = filter.tag_id {
            qb.push(" AND id IN (SELECT backlink_id FROM backlink_tags WHERE organization_id = ");
            qb.push_bind(scope.organization_id());
            qb.push(" AND tag_id = ");
            qb.push_bind(tag);
            qb.push(")");
        }
        if let Some(s) = filter.since {
            qb.push(" AND COALESCE(first_checked_at, created_at) >= ");
            qb.push_bind(s);
        }
        qb.push(" ORDER BY created_at DESC LIMIT ");
        qb.push_bind(limit.clamp(1, 200));
        if filter.offset > 0 {
            qb.push(" OFFSET ");
            qb.push_bind(filter.offset.max(0));
        }

        Ok(qb.build_query_as::<Backlink>().fetch_all(pool).await?)
    }

    /// Minimal rows for the footprint auto-tagger: id + the fields the classifier
    /// reads (referring domain, source URL, discovered anchor).
    #[derive(Debug, Clone, sqlx::FromRow)]
    pub struct AutoTagRow {
        pub id: Uuid,
        pub source_domain: String,
        pub source_url: String,
        pub anchor: String,
    }

    pub async fn list_for_autotag(
        pool: &PgPool,
        scope: &OrgScope,
        project_id: Uuid,
    ) -> Result<Vec<AutoTagRow>, DbError> {
        Ok(sqlx::query_as::<_, AutoTagRow>(
            "SELECT id, source_domain, source_url, \
                    COALESCE(expected_anchor_text, '') AS anchor \
             FROM backlinks \
             WHERE project_id = $1 AND organization_id = $2 \
               AND subject = 'own' AND tier = 1 AND archived_at IS NULL \
             LIMIT 5000",
        )
        .bind(project_id)
        .bind(scope.organization_id())
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

    /// Backlinks for CSV export (§36): the whole org, one project, or one
    /// client, newest first, capped. Tenant-scoped. `QueryBuilder`, so the
    /// optional filters are bound params (S9-safe) rather than string-built.
    pub async fn list_for_export(
        pool: &PgPool,
        scope: &OrgScope,
        project_id: Option<Uuid>,
        client_id: Option<Uuid>,
        limit: i64,
    ) -> Result<Vec<Backlink>, DbError> {
        let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new(concat!(
            "SELECT ",
            cols!(),
            " FROM backlinks WHERE organization_id = "
        ));
        qb.push_bind(scope.organization_id());
        qb.push(" AND archived_at IS NULL AND subject = 'own' AND tier = 1");
        if let Some(pid) = project_id {
            qb.push(" AND project_id = ");
            qb.push_bind(pid);
        }
        if let Some(cid) = client_id {
            qb.push(" AND project_id IN (SELECT id FROM projects WHERE organization_id = ");
            qb.push_bind(scope.organization_id());
            qb.push(" AND client_id = ");
            qb.push_bind(cid);
            qb.push(")");
        }
        qb.push(" ORDER BY created_at DESC LIMIT ");
        qb.push_bind(limit.clamp(1, 50_000));
        Ok(qb.build_query_as::<Backlink>().fetch_all(pool).await?)
    }

    /// Competitor backlinks for one competitor (subject='competitor'), newest
    /// first, capped. Tenant-scoped.
    pub async fn search_competitor(
        pool: &PgPool,
        scope: &OrgScope,
        competitor_id: Uuid,
        limit: i64,
    ) -> Result<Vec<Backlink>, DbError> {
        Ok(sqlx::query_as::<_, Backlink>(concat!(
            "SELECT ",
            cols!(),
            " FROM backlinks WHERE organization_id = $1 AND competitor_id = $2 \
               AND subject = 'competitor' AND archived_at IS NULL \
             ORDER BY created_at DESC LIMIT $3"
        ))
        .bind(scope.organization_id())
        .bind(competitor_id)
        .bind(limit.clamp(1, 500))
        .fetch_all(pool)
        .await?)
    }

    /// Tier-2 backlinks reinforcing a given Tier-1 backlink (its children in the
    /// graph), newest first. Tenant-scoped.
    pub async fn search_tier2(
        pool: &PgPool,
        scope: &OrgScope,
        parent_backlink_id: Uuid,
        limit: i64,
    ) -> Result<Vec<Backlink>, DbError> {
        Ok(sqlx::query_as::<_, Backlink>(concat!(
            "SELECT ",
            cols!(),
            " FROM backlinks WHERE organization_id = $1 AND parent_backlink_id = $2 \
               AND archived_at IS NULL ORDER BY created_at DESC LIMIT $3"
        ))
        .bind(scope.organization_id())
        .bind(parent_backlink_id)
        .bind(limit.clamp(1, 500))
        .fetch_all(pool)
        .await?)
    }

    /// Count active backlinks in an org — for plan-limit enforcement (§39).
    pub async fn count_active(pool: &PgPool, scope: &OrgScope) -> Result<i64, DbError> {
        let (n,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM backlinks \
             WHERE organization_id = $1 AND archived_at IS NULL AND subject = 'own' AND tier = 1",
        )
        .bind(scope.organization_id())
        .fetch_one(pool)
        .await?;
        Ok(n)
    }

    /// Soft-archive a backlink (§36): stamp `archived_at` and clear
    /// `next_check_at` so the scheduler stops checking it (the schema CHECK
    /// requires next_check_at to be NULL once archived). Keeps the last known
    /// state and history. Tenant-scoped; a cross-tenant or unknown id (or one
    /// already archived) yields `NotFound`.
    pub async fn archive(pool: &PgPool, scope: &OrgScope, id: Uuid) -> Result<(), DbError> {
        let row: Option<(Uuid,)> = sqlx::query_as(
            "UPDATE backlinks SET archived_at = now(), next_check_at = NULL \
             WHERE id = $1 AND organization_id = $2 AND archived_at IS NULL RETURNING id",
        )
        .bind(id)
        .bind(scope.organization_id())
        .fetch_optional(pool)
        .await?;
        row.map(|_| ()).ok_or(DbError::NotFound)
    }

    /// Restore an archived backlink: clear `archived_at` and make it due now so
    /// the scheduler picks it back up. `NotFound` if it is not archived or not
    /// in this org.
    pub async fn restore(pool: &PgPool, scope: &OrgScope, id: Uuid) -> Result<(), DbError> {
        let row: Option<(Uuid,)> = sqlx::query_as(
            "UPDATE backlinks SET archived_at = NULL, next_check_at = now() \
             WHERE id = $1 AND organization_id = $2 AND archived_at IS NOT NULL RETURNING id",
        )
        .bind(id)
        .bind(scope.organization_id())
        .fetch_optional(pool)
        .await?;
        row.map(|_| ()).ok_or(DbError::NotFound)
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
        /// "text" | "image" when a link was detected this cycle, else None.
        pub link_type: Option<String>,
        pub image_alt: Option<String>,
        /// Target fetch result (only when the link was found and target-checking
        /// is on); None otherwise, and then the stored value is preserved.
        pub target_http_status: Option<i32>,
        pub target_final_url: Option<String>,
    }

    /// One check row to append.
    pub struct CheckWrite {
        pub crawl_id: Uuid,
        pub found: bool,
        pub status_after: String,
        pub source_status: Option<i32>,
        pub target_status: Option<i32>,
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
                final_url, duration_ms, bytes, error_kind, target_status) \
             VALUES ($1,$2,$3,$4,$5,$6::backlink_status,$7,$8,$9,$10,$11,$12,$13,$14,$15) \
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
        .bind(check.target_status)
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
                failure_count = failure_count + $14, \
                link_type = $15, image_alt = $16, \
                target_http_status = COALESCE($17, target_http_status), \
                target_final_url = COALESCE($18, target_final_url), \
                lost_at = CASE \
                    WHEN $2 = 'lost' THEN COALESCE(lost_at, now()) \
                    WHEN $2 = 'live' THEN NULL \
                    ELSE lost_at END \
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
        .bind(&transition.link_type)
        .bind(&transition.image_alt)
        .bind(transition.target_http_status)
        .bind(&transition.target_final_url)
        .execute(&mut *tx)
        .await
        .map_err(DbError::from_insert)?;

        tx.commit().await?;
        Ok(true)
    }
}

// ─── Link occurrences: every physical link on a source page (docs/06 §3.4) ───
pub mod link_occurrences {
    use super::*;
    use crate::entities::LinkOccurrence;

    /// One observed occurrence, ready to upsert.
    pub struct OccurrenceInput {
        pub fingerprint: String,
        pub anchor_text: Option<String>,
        pub link_type: Option<String>,
        pub image_alt: Option<String>,
        pub href_raw: Option<String>,
        pub rel_raw: Vec<String>,
        pub is_nofollow: bool,
        pub is_sponsored: bool,
        pub is_ugc: bool,
        pub placement: String,
        pub surrounding_text: Option<String>,
        pub position: Option<i32>,
    }

    /// Upsert every occurrence seen on one crawl, then mark occurrences that were
    /// present before but not seen this time as removed. Transactional and
    /// idempotent per `seen_at` (docs/07 §4): the touched rows get
    /// `last_seen_at = seen_at`, so the mark-removed step (`last_seen_at < seen_at`)
    /// never touches them and a re-run is a no-op.
    pub async fn sync(
        pool: &PgPool,
        organization_id: Uuid,
        backlink_id: Uuid,
        occs: &[OccurrenceInput],
        seen_at: DateTime<Utc>,
    ) -> Result<(), DbError> {
        let mut tx = pool.begin().await?;
        for o in occs {
            // Reliable LINK_MOVED: same occurrence (fingerprint), different region.
            let existing: Option<(String,)> = sqlx::query_as(
                "SELECT placement FROM link_occurrences \
                 WHERE backlink_id = $1 AND fingerprint = $2 AND removed_at IS NULL",
            )
            .bind(backlink_id)
            .bind(&o.fingerprint)
            .fetch_optional(&mut *tx)
            .await?;
            if let Some((old,)) = existing
                && old != o.placement
            {
                sqlx::query(
                    "INSERT INTO backlink_events (id, backlink_id, organization_id, check_id, \
                        event_type, previous_data, current_data) \
                     VALUES ($1,$2,$3,NULL,'link_moved'::event_type,$4,$5)",
                )
                .bind(Uuid::now_v7())
                .bind(backlink_id)
                .bind(organization_id)
                .bind(serde_json::Value::String(old))
                .bind(serde_json::Value::String(o.placement.clone()))
                .execute(&mut *tx)
                .await
                .map_err(DbError::from_insert)?;
            }
            sqlx::query(
                "INSERT INTO link_occurrences (id, organization_id, backlink_id, fingerprint, \
                    anchor_text, link_type, image_alt, href_raw, rel_raw, \
                    is_nofollow, is_sponsored, is_ugc, placement, surrounding_text, position, \
                    first_seen_at, last_seen_at, removed_at) \
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$16,NULL) \
                 ON CONFLICT (backlink_id, fingerprint) DO UPDATE SET \
                    anchor_text = EXCLUDED.anchor_text, link_type = EXCLUDED.link_type, \
                    image_alt = EXCLUDED.image_alt, href_raw = EXCLUDED.href_raw, \
                    rel_raw = EXCLUDED.rel_raw, is_nofollow = EXCLUDED.is_nofollow, \
                    is_sponsored = EXCLUDED.is_sponsored, is_ugc = EXCLUDED.is_ugc, \
                    placement = EXCLUDED.placement, surrounding_text = EXCLUDED.surrounding_text, \
                    position = EXCLUDED.position, last_seen_at = EXCLUDED.last_seen_at, \
                    removed_at = NULL",
            )
            .bind(Uuid::now_v7())
            .bind(organization_id)
            .bind(backlink_id)
            .bind(&o.fingerprint)
            .bind(&o.anchor_text)
            .bind(&o.link_type)
            .bind(&o.image_alt)
            .bind(&o.href_raw)
            .bind(&o.rel_raw)
            .bind(o.is_nofollow)
            .bind(o.is_sponsored)
            .bind(o.is_ugc)
            .bind(&o.placement)
            .bind(&o.surrounding_text)
            .bind(o.position)
            .bind(seen_at)
            .execute(&mut *tx)
            .await
            .map_err(DbError::from_insert)?;
        }
        sqlx::query(
            "UPDATE link_occurrences SET removed_at = $2 \
             WHERE backlink_id = $1 AND removed_at IS NULL AND last_seen_at < $2",
        )
        .bind(backlink_id)
        .bind(seen_at)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Live occurrences of one backlink (org-scoped by the caller).
    pub async fn list(
        pool: &PgPool,
        organization_id: Uuid,
        backlink_id: Uuid,
    ) -> Result<Vec<LinkOccurrence>, DbError> {
        Ok(sqlx::query_as::<_, LinkOccurrence>(
            "SELECT id, organization_id, backlink_id, fingerprint, anchor_text, link_type, \
                image_alt, href_raw, rel_raw, is_nofollow, is_sponsored, is_ugc, placement, \
                surrounding_text, position, first_seen_at, last_seen_at, removed_at \
             FROM link_occurrences \
             WHERE organization_id = $1 AND backlink_id = $2 AND removed_at IS NULL \
             ORDER BY position NULLS LAST, last_seen_at DESC",
        )
        .bind(organization_id)
        .bind(backlink_id)
        .fetch_all(pool)
        .await?)
    }

    /// Count of live occurrences across the org (dashboard metric).
    pub async fn count_active(pool: &PgPool, organization_id: Uuid) -> Result<i64, DbError> {
        let (n,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM link_occurrences \
             WHERE organization_id = $1 AND removed_at IS NULL",
        )
        .bind(organization_id)
        .fetch_one(pool)
        .await?;
        Ok(n)
    }
}

// ─── Competitors: tracked competitor domains + their backlinks ───────────────
pub mod competitors {
    use super::*;
    use crate::entities::ProjectCompetitor;
    use backlio_core::{NormalizeOptions, host_of, normalize};

    fn norm_domain(input: &str) -> Option<String> {
        let url = if input.contains("://") {
            input.to_string()
        } else {
            format!("https://{}", input.trim())
        };
        host_of(&normalize(&url, None, NormalizeOptions::default())?)
    }

    macro_rules! competitor_cols {
        () => {
            "id, organization_id, project_id, domain, normalized_domain, \
             enabled, first_scanned_at, last_scanned_at, created_at, updated_at"
        };
    }

    /// Add a competitor to a project (idempotent on (project, normalized_domain)).
    pub async fn create(
        pool: &PgPool,
        scope: &OrgScope,
        id: Uuid,
        project_id: Uuid,
        domain: &str,
    ) -> Result<ProjectCompetitor, DbError> {
        let normalized = norm_domain(domain)
            .ok_or_else(|| DbError::Config("invalid competitor domain".into()))?;
        let inserted: Option<ProjectCompetitor> = sqlx::query_as::<_, ProjectCompetitor>(concat!(
            "INSERT INTO project_competitors \
                (id, organization_id, project_id, domain, normalized_domain) \
             VALUES ($1,$2,$3,$4,$5) \
             ON CONFLICT (project_id, normalized_domain) DO NOTHING RETURNING ",
            competitor_cols!()
        ))
        .bind(id)
        .bind(scope.organization_id())
        .bind(project_id)
        .bind(domain.trim())
        .bind(&normalized)
        .fetch_optional(pool)
        .await
        .map_err(DbError::from_insert)?;
        if let Some(c) = inserted {
            return Ok(c);
        }
        // Already present — return the existing row.
        sqlx::query_as::<_, ProjectCompetitor>(concat!(
            "SELECT ",
            competitor_cols!(),
            " FROM project_competitors \
             WHERE organization_id = $1 AND project_id = $2 AND normalized_domain = $3"
        ))
        .bind(scope.organization_id())
        .bind(project_id)
        .bind(&normalized)
        .fetch_one(pool)
        .await
        .map_err(DbError::from_fetch)
    }

    pub async fn list(
        pool: &PgPool,
        scope: &OrgScope,
        project_id: Uuid,
    ) -> Result<Vec<ProjectCompetitor>, DbError> {
        Ok(sqlx::query_as::<_, ProjectCompetitor>(concat!(
            "SELECT ",
            competitor_cols!(),
            " FROM project_competitors \
             WHERE organization_id = $1 AND project_id = $2 ORDER BY created_at"
        ))
        .bind(scope.organization_id())
        .bind(project_id)
        .fetch_all(pool)
        .await?)
    }

    pub async fn get(
        pool: &PgPool,
        scope: &OrgScope,
        id: Uuid,
    ) -> Result<ProjectCompetitor, DbError> {
        sqlx::query_as::<_, ProjectCompetitor>(concat!(
            "SELECT ",
            competitor_cols!(),
            " FROM project_competitors WHERE id = $1 AND organization_id = $2"
        ))
        .bind(id)
        .bind(scope.organization_id())
        .fetch_one(pool)
        .await
        .map_err(DbError::from_fetch)
    }

    pub async fn delete(pool: &PgPool, scope: &OrgScope, id: Uuid) -> Result<bool, DbError> {
        let r =
            sqlx::query("DELETE FROM project_competitors WHERE id = $1 AND organization_id = $2")
                .bind(id)
                .bind(scope.organization_id())
                .execute(pool)
                .await?;
        Ok(r.rows_affected() > 0)
    }

    /// Backlink gap / link intersect (§4): referring domains that link to at
    /// least `min_competitors` of this project's competitors but NOT to the
    /// project's own site. These are the opportunity domains. Returns
    /// (source_domain, competitor_count), strongest first.
    pub async fn gap_analysis(
        pool: &PgPool,
        scope: &OrgScope,
        project_id: Uuid,
        min_competitors: i64,
        limit: i64,
    ) -> Result<Vec<(String, i64)>, DbError> {
        Ok(sqlx::query_as::<_, (String, i64)>(
            "SELECT b.source_domain, count(DISTINCT b.competitor_id) AS competitor_count \
             FROM backlinks b \
             JOIN project_competitors pc \
               ON pc.organization_id = b.organization_id AND pc.id = b.competitor_id \
             WHERE b.organization_id = $1 AND b.subject = 'competitor' \
               AND b.archived_at IS NULL AND pc.project_id = $2 \
               AND b.source_domain NOT IN ( \
                   SELECT source_domain FROM backlinks \
                   WHERE organization_id = $1 AND subject = 'own' AND tier = 1 \
                     AND project_id = $2 AND archived_at IS NULL) \
             GROUP BY b.source_domain \
             HAVING count(DISTINCT b.competitor_id) >= $3 \
             ORDER BY competitor_count DESC, b.source_domain \
             LIMIT $4",
        )
        .bind(scope.organization_id())
        .bind(project_id)
        .bind(min_competitors.max(1))
        .bind(limit.clamp(1, 1000))
        .fetch_all(pool)
        .await?)
    }

    /// Ingest one discovered competitor backlink. Returns (backlink_id, is_new).
    /// On a genuinely new relation it emits competitor_new_backlink and, when
    /// this is the first backlink seen from that page / domain for the
    /// competitor, competitor_new_referring_page / _referring_domain — all in one
    /// transaction so a re-run never double-emits.
    pub async fn record_backlink(
        pool: &PgPool,
        scope: &OrgScope,
        competitor: &ProjectCompetitor,
        source_url: &str,
        target_url: &str,
        anchor: Option<&str>,
        rel_type: &str,
    ) -> Result<(Uuid, bool), DbError> {
        record_backlink_with_meta(
            pool, scope, competitor, source_url, target_url, anchor, rel_type, None,
        )
        .await
    }

    /// As `record_backlink`, pre-populating first_checked_at / last_seen_at /
    /// source_http_status from provider metadata when supplied (spec §21).
    #[allow(clippy::too_many_arguments)]
    pub async fn record_backlink_with_meta(
        pool: &PgPool,
        scope: &OrgScope,
        competitor: &ProjectCompetitor,
        source_url: &str,
        target_url: &str,
        anchor: Option<&str>,
        rel_type: &str,
        meta: Option<&super::DiscoveredMeta>,
    ) -> Result<(Uuid, bool), DbError> {
        let opts = NormalizeOptions::default();
        let source_norm = normalize(source_url, None, opts)
            .ok_or_else(|| DbError::Config("invalid source_url".into()))?;
        let target_norm = normalize(target_url, None, opts)
            .ok_or_else(|| DbError::Config("invalid target_url".into()))?;
        let source_domain = host_of(&source_norm).unwrap_or_default();
        let target_domain = host_of(&target_norm).unwrap_or_default();
        let org = scope.organization_id();
        let (m_first, m_last, m_status) = match meta {
            Some(m) => (m.first_seen, m.last_seen, m.source_status),
            None => (None, None, None),
        };

        let mut tx = pool.begin().await?;

        let existing: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM backlinks WHERE organization_id = $1 AND project_id = $2 \
               AND subject = 'competitor' AND source_url_normalized = $3 \
               AND target_url_normalized = $4 AND archived_at IS NULL",
        )
        .bind(org)
        .bind(competitor.project_id)
        .bind(&source_norm)
        .bind(&target_norm)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some((id,)) = existing {
            tx.commit().await?;
            return Ok((id, false));
        }

        // Firsts (scoped to this competitor), measured before we insert.
        let (page_seen,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM backlinks WHERE organization_id = $1 AND competitor_id = $2 \
               AND source_url_normalized = $3 AND archived_at IS NULL",
        )
        .bind(org)
        .bind(competitor.id)
        .bind(&source_norm)
        .fetch_one(&mut *tx)
        .await?;
        let (domain_seen,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM backlinks WHERE organization_id = $1 AND competitor_id = $2 \
               AND source_domain = $3 AND archived_at IS NULL",
        )
        .bind(org)
        .bind(competitor.id)
        .bind(&source_domain)
        .fetch_one(&mut *tx)
        .await?;

        let id = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO backlinks (id, organization_id, project_id, \
                source_url, source_url_normalized, source_domain, \
                target_url, target_url_normalized, target_domain, \
                expected_anchor_text, expected_rel_type, subject, competitor_id, \
                first_checked_at, last_seen_at, source_http_status, next_check_at) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11::rel_type,'competitor',$12,$13,$14,$15, now())",
        )
        .bind(id)
        .bind(org)
        .bind(competitor.project_id)
        .bind(source_url)
        .bind(&source_norm)
        .bind(&source_domain)
        .bind(target_url)
        .bind(&target_norm)
        .bind(&target_domain)
        .bind(anchor)
        .bind(rel_type)
        .bind(competitor.id)
        .bind(m_first)
        .bind(m_last)
        .bind(m_status)
        .execute(&mut *tx)
        .await
        .map_err(DbError::from_insert)?;

        let payload = serde_json::json!({
            "competitor_id": competitor.id,
            "competitor_domain": competitor.normalized_domain,
            "source_domain": source_domain,
            "source_url": source_url,
            "target_url": target_url,
        });
        let mut to_emit = vec!["competitor_new_backlink"];
        if domain_seen == 0 {
            to_emit.push("competitor_new_referring_domain");
        }
        if page_seen == 0 {
            to_emit.push("competitor_new_referring_page");
        }
        for et in to_emit {
            sqlx::query(
                "INSERT INTO backlink_events \
                    (id, backlink_id, organization_id, check_id, event_type, previous_data, current_data) \
                 VALUES ($1,$2,$3,NULL,$4::event_type,NULL,$5)",
            )
            .bind(Uuid::now_v7())
            .bind(id)
            .bind(org)
            .bind(et)
            .bind(&payload)
            .execute(&mut *tx)
            .await
            .map_err(DbError::from_insert)?;
        }

        // Phase 2 alert: a NEW referring domain to a competitor is notable enough
        // to raise an in-app notification immediately (org-wide, deduped). Per-
        // backlink noise is left to a digest. The notifications table already
        // carries channel/digest columns for email/slack/webhook fan-out later.
        if domain_seen == 0 {
            let dedupe = format!("comp_dom:{}:{}", competitor.id, source_domain);
            sqlx::query(
                "INSERT INTO notifications \
                    (id, organization_id, backlink_id, kind, title, body, data, dedupe_key) \
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8) \
                 ON CONFLICT (dedupe_key) WHERE dedupe_key IS NOT NULL DO NOTHING",
            )
            .bind(Uuid::now_v7())
            .bind(org)
            .bind(id)
            .bind("competitor_new_referring_domain")
            .bind(format!(
                "New referring domain for {}",
                competitor.normalized_domain
            ))
            .bind(format!(
                "{source_domain} now links to {}",
                competitor.normalized_domain
            ))
            .bind(&payload)
            .bind(&dedupe)
            .execute(&mut *tx)
            .await
            .map_err(DbError::from_insert)?;
        }

        tx.commit().await?;
        Ok((id, true))
    }
}

// ─── Tiers: Tier-2 backlinks that reinforce a Tier-1 edge ────────────────────
pub mod tiers {
    use super::*;
    use crate::entities::Backlink;
    use backlio_core::{NormalizeOptions, host_of, normalize};

    /// Ingest a Tier-2 backlink: a page linking to `parent`'s SOURCE page (which
    /// is itself a Tier-1 backlink to our site). Stored as own/tier=parent+1 with
    /// `parent_backlink_id` set. Returns (id, is_new); dedups the relation.
    pub async fn record_tier2(
        pool: &PgPool,
        scope: &OrgScope,
        parent: &Backlink,
        source_url: &str,
        anchor: Option<&str>,
        rel_type: &str,
    ) -> Result<(Uuid, bool), DbError> {
        let opts = NormalizeOptions::default();
        // The Tier-2 link points AT the Tier-1 source page.
        let target_url = parent.source_url.clone();
        let source_norm = normalize(source_url, None, opts)
            .ok_or_else(|| DbError::Config("invalid source_url".into()))?;
        let target_norm = normalize(&target_url, None, opts)
            .ok_or_else(|| DbError::Config("invalid tier-1 source url".into()))?;
        let source_domain = host_of(&source_norm).unwrap_or_default();
        let target_domain = host_of(&target_norm).unwrap_or_default();
        let org = scope.organization_id();

        let mut tx = pool.begin().await?;
        let existing: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM backlinks WHERE organization_id = $1 AND project_id = $2 \
               AND parent_backlink_id = $3 AND source_url_normalized = $4 \
               AND target_url_normalized = $5 AND archived_at IS NULL",
        )
        .bind(org)
        .bind(parent.project_id)
        .bind(parent.id)
        .bind(&source_norm)
        .bind(&target_norm)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some((id,)) = existing {
            tx.commit().await?;
            return Ok((id, false));
        }

        let id = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO backlinks (id, organization_id, project_id, \
                source_url, source_url_normalized, source_domain, \
                target_url, target_url_normalized, target_domain, \
                expected_anchor_text, expected_rel_type, subject, tier, parent_backlink_id, next_check_at) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11::rel_type,'own',$12,$13, now())",
        )
        .bind(id)
        .bind(org)
        .bind(parent.project_id)
        .bind(source_url)
        .bind(&source_norm)
        .bind(&source_domain)
        .bind(&target_url)
        .bind(&target_norm)
        .bind(&target_domain)
        .bind(anchor)
        .bind(rel_type)
        .bind(parent.tier + 1)
        .bind(parent.id)
        .execute(&mut *tx)
        .await
        .map_err(DbError::from_insert)?;

        tx.commit().await?;
        Ok((id, true))
    }
}

// ─── Graph: derive nodes/edges from backlinks (backlink-intelligence §6, §12) ─
pub mod graph {
    use super::*;

    /// One edge's raw fields — the API turns these into domain/url graph nodes.
    #[derive(sqlx::FromRow)]
    pub struct EdgeRow {
        pub id: Uuid,
        pub source_domain: String,
        pub source_url: String,
        pub target_domain: String,
        pub target_url: String,
        pub tier: i16,
        pub parent_backlink_id: Option<Uuid>,
        pub status: String,
    }

    /// The project's own backlink edges (all tiers), lowest tier first, capped.
    /// No new node table — the graph is derived (docs §6: URLs suffice).
    pub async fn edges(
        pool: &PgPool,
        scope: &OrgScope,
        project_id: Uuid,
        limit: i64,
    ) -> Result<Vec<EdgeRow>, DbError> {
        Ok(sqlx::query_as::<_, EdgeRow>(
            "SELECT id, source_domain, source_url, target_domain, target_url, tier, \
                parent_backlink_id, status::text AS status \
             FROM backlinks \
             WHERE organization_id = $1 AND project_id = $2 AND subject = 'own' \
               AND archived_at IS NULL \
             ORDER BY tier, created_at LIMIT $3",
        )
        .bind(scope.organization_id())
        .bind(project_id)
        .bind(limit.clamp(1, 20_000))
        .fetch_all(pool)
        .await?)
    }
}

// ─── Discovery seeds: referring domains to expand-crawl (spec §21) ───────────
pub mod seeds {
    use super::*;
    use crate::entities::DiscoverySeed;
    use backlio_core::{NormalizeOptions, host_of, normalize};

    macro_rules! seed_cols {
        () => {
            "id, organization_id, project_id, domain, normalized_domain, source, status, \
             pages_crawled, backlinks_found, last_crawled_at, next_crawl_at, last_error, \
             created_at, updated_at"
        };
    }

    fn norm_domain(input: &str) -> Option<String> {
        let url = if input.contains("://") {
            input.to_string()
        } else {
            format!("https://{}", input.trim())
        };
        host_of(&normalize(&url, None, NormalizeOptions::default())?)
    }

    /// Add a seed domain to crawl (idempotent per project + domain).
    pub async fn create(
        pool: &PgPool,
        scope: &OrgScope,
        id: Uuid,
        project_id: Uuid,
        domain: &str,
        source: &str,
    ) -> Result<Option<DiscoverySeed>, DbError> {
        let normalized = match norm_domain(domain) {
            Some(d) if !d.is_empty() => d,
            _ => return Ok(None),
        };
        sqlx::query_as::<_, DiscoverySeed>(concat!(
            "INSERT INTO discovery_seeds \
                (id, organization_id, project_id, domain, normalized_domain, source) \
             VALUES ($1,$2,$3,$4,$5,$6) \
             ON CONFLICT (project_id, normalized_domain) DO NOTHING RETURNING ",
            seed_cols!()
        ))
        .bind(id)
        .bind(scope.organization_id())
        .bind(project_id)
        .bind(domain.trim())
        .bind(&normalized)
        .bind(source)
        .fetch_optional(pool)
        .await
        .map_err(DbError::from_insert)
    }

    pub async fn list(
        pool: &PgPool,
        scope: &OrgScope,
        project_id: Uuid,
    ) -> Result<Vec<DiscoverySeed>, DbError> {
        Ok(sqlx::query_as::<_, DiscoverySeed>(concat!(
            "SELECT ",
            seed_cols!(),
            " FROM discovery_seeds WHERE organization_id = $1 AND project_id = $2 \
             ORDER BY created_at DESC"
        ))
        .bind(scope.organization_id())
        .bind(project_id)
        .fetch_all(pool)
        .await?)
    }

    /// Claim due seeds for crawling (cross-tenant; the worker runs them).
    pub async fn claim_due(pool: &PgPool, limit: i64) -> Result<Vec<DiscoverySeed>, DbError> {
        Ok(sqlx::query_as::<_, DiscoverySeed>(concat!(
            "UPDATE discovery_seeds SET status='crawling', updated_at=now() \
             WHERE id IN ( \
                SELECT id FROM discovery_seeds \
                WHERE status IN ('pending','done') AND next_crawl_at <= now() \
                ORDER BY next_crawl_at FOR UPDATE SKIP LOCKED LIMIT $1 \
             ) RETURNING ",
            seed_cols!()
        ))
        .bind(limit.clamp(1, 50))
        .fetch_all(pool)
        .await?)
    }

    /// Record a crawl result and reschedule.
    pub async fn finish(
        pool: &PgPool,
        id: Uuid,
        pages: i32,
        found: i32,
        next_in_secs: i64,
        error: Option<&str>,
    ) -> Result<(), DbError> {
        sqlx::query(
            "UPDATE discovery_seeds SET \
                status = CASE WHEN $5::text IS NULL THEN 'done' ELSE 'error' END, \
                pages_crawled = $2, backlinks_found = $3, last_crawled_at = now(), \
                next_crawl_at = now() + make_interval(secs => $4), last_error = $5 \
             WHERE id = $1",
        )
        .bind(id)
        .bind(pages)
        .bind(found)
        .bind(next_in_secs)
        .bind(error)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Insert a discovered own Tier-1 backlink if the relation is new.
    /// Returns whether it was newly created. Idempotent per project + urls.
    pub async fn record_backlink(
        pool: &PgPool,
        organization_id: Uuid,
        project_id: Uuid,
        source_url: &str,
        target_url: &str,
        anchor: Option<&str>,
        rel_type: &str,
    ) -> Result<bool, DbError> {
        record_backlink_with_meta(
            pool,
            organization_id,
            project_id,
            source_url,
            target_url,
            anchor,
            rel_type,
            None,
        )
        .await
    }

    /// As `record_backlink`, but pre-populates first_checked_at / last_seen_at /
    /// source_http_status from provider metadata when supplied (spec §21).
    #[allow(clippy::too_many_arguments)]
    pub async fn record_backlink_with_meta(
        pool: &PgPool,
        organization_id: Uuid,
        project_id: Uuid,
        source_url: &str,
        target_url: &str,
        anchor: Option<&str>,
        rel_type: &str,
        meta: Option<&super::DiscoveredMeta>,
    ) -> Result<bool, DbError> {
        let opts = NormalizeOptions::default();
        let source_norm = match normalize(source_url, None, opts) {
            Some(s) => s,
            None => return Ok(false),
        };
        let target_norm = match normalize(target_url, None, opts) {
            Some(s) => s,
            None => return Ok(false),
        };
        let source_domain = host_of(&source_norm).unwrap_or_default();
        let target_domain = host_of(&target_norm).unwrap_or_default();
        let (first_seen, last_seen, source_status) = match meta {
            Some(m) => (m.first_seen, m.last_seen, m.source_status),
            None => (None, None, None),
        };

        let mut tx = pool.begin().await?;
        let existing: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM backlinks WHERE organization_id = $1 AND project_id = $2 \
               AND subject = 'own' AND source_url_normalized = $3 \
               AND target_url_normalized = $4 AND archived_at IS NULL",
        )
        .bind(organization_id)
        .bind(project_id)
        .bind(&source_norm)
        .bind(&target_norm)
        .fetch_optional(&mut *tx)
        .await?;
        if existing.is_some() {
            tx.commit().await?;
            return Ok(false);
        }
        sqlx::query(
            "INSERT INTO backlinks (id, organization_id, project_id, \
                source_url, source_url_normalized, source_domain, \
                target_url, target_url_normalized, target_domain, \
                expected_anchor_text, expected_rel_type, subject, tier, \
                first_checked_at, last_seen_at, source_http_status, next_check_at) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11::rel_type,'own',1,$12,$13,$14, now())",
        )
        .bind(Uuid::now_v7())
        .bind(organization_id)
        .bind(project_id)
        .bind(source_url)
        .bind(&source_norm)
        .bind(&source_domain)
        .bind(target_url)
        .bind(&target_norm)
        .bind(&target_domain)
        .bind(anchor)
        .bind(rel_type)
        .bind(first_seen)
        .bind(last_seen)
        .bind(source_status)
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

        // Only monitor workspaces that are actually entitled: not suspended, and
        // with an active/past_due subscription or an un-expired trial (docs/20).
        // Expired trials, canceled subscriptions and suspended orgs are skipped —
        // the cost-protection gate (their data is kept, just not crawled).
        let due: Vec<(Uuid, String, String)> = sqlx::query_as(
            "SELECT b.id, b.source_url, b.source_url_normalized FROM backlinks b \
             WHERE b.archived_at IS NULL AND b.next_check_at IS NOT NULL AND b.next_check_at <= now() \
               AND EXISTS ( \
                   SELECT 1 FROM organizations o \
                   JOIN subscriptions s ON s.organization_id = o.id \
                   WHERE o.id = b.organization_id \
                     AND o.suspended_at IS NULL \
                     AND ( s.status IN ('active','past_due') \
                        OR (s.status = 'trialing' AND s.trial_end > now()) ) ) \
             ORDER BY b.next_check_at LIMIT $1 FOR UPDATE SKIP LOCKED",
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

    /// Common WHERE for own/tier-1/non-archived backlinks, with an optional
    /// "acquired since" cutoff on first-seen (COALESCE(first_checked_at, created_at)).
    /// Common WHERE for own/tier-1/non-archived backlinks (aliased `b`), with an
    /// optional acquisition-period cutoff on first-seen.
    fn scoped(
        prefix: &str,
        scope: &OrgScope,
        since: Option<DateTime<Utc>>,
    ) -> sqlx::QueryBuilder<sqlx::Postgres> {
        let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new(prefix.to_string());
        qb.push(" WHERE b.organization_id = ");
        qb.push_bind(scope.organization_id());
        qb.push(" AND b.archived_at IS NULL AND b.subject = 'own' AND b.tier = 1");
        if let Some(s) = since {
            qb.push(" AND COALESCE(b.first_checked_at, b.created_at) >= ");
            qb.push_bind(s);
        }
        qb
    }

    /// (status, count) for own backlinks, optionally within an acquisition period.
    pub async fn status_counts(
        pool: &PgPool,
        scope: &OrgScope,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<(String, i64)>, DbError> {
        let mut qb = scoped(
            "SELECT b.status::text, count(*) FROM backlinks b",
            scope,
            since,
        );
        qb.push(" GROUP BY b.status");
        Ok(qb.build_query_as::<(String, i64)>().fetch_all(pool).await?)
    }

    /// (dofollow, nofollow) split — detected when known, else the expected rel.
    pub async fn rel_split(
        pool: &PgPool,
        scope: &OrgScope,
        since: Option<DateTime<Utc>>,
    ) -> Result<(i64, i64), DbError> {
        let mut qb = scoped(
            "SELECT \
               count(*) FILTER (WHERE COALESCE(b.detected_is_dofollow, b.expected_rel_type::text = 'dofollow'))::bigint, \
               count(*) FILTER (WHERE NOT COALESCE(b.detected_is_dofollow, b.expected_rel_type::text = 'dofollow'))::bigint \
             FROM backlinks b",
            scope,
            since,
        );
        Ok(qb.build_query_as::<(i64, i64)>().fetch_one(pool).await?)
    }

    /// (tag_name, count) for the auto-tagger link-type tags only.
    pub async fn type_counts(
        pool: &PgPool,
        scope: &OrgScope,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<(String, i64)>, DbError> {
        let mut qb = scoped(
            "SELECT t.name::text, count(DISTINCT b.id)::bigint \
             FROM backlinks b \
             JOIN backlink_tags bt ON bt.backlink_id = b.id \
             JOIN tags t ON t.id = bt.tag_id \
                AND lower(t.name) IN ('comment','pbn','forum-profile','self-built','earned')",
            scope,
            since,
        );
        qb.push(" GROUP BY t.name");
        Ok(qb.build_query_as::<(String, i64)>().fetch_all(pool).await?)
    }

    /// (referring domains, referring pages): distinct source domains and distinct
    /// source pages across the org's live (non-archived) backlinks.
    pub async fn referring_counts(pool: &PgPool, scope: &OrgScope) -> Result<(i64, i64), DbError> {
        let row: (i64, i64) = sqlx::query_as(
            "SELECT count(DISTINCT source_domain), count(DISTINCT source_url_normalized) \
             FROM backlinks WHERE organization_id = $1 AND archived_at IS NULL AND subject = 'own' AND tier = 1",
        )
        .bind(scope.organization_id())
        .fetch_one(pool)
        .await?;
        Ok(row)
    }

    /// (event_type, count) over a time window — powers the "what changed
    /// recently" dashboard view (§34). Reads the append-only events table.
    pub async fn event_counts(
        pool: &PgPool,
        scope: &OrgScope,
        since: DateTime<Utc>,
    ) -> Result<Vec<(String, i64)>, DbError> {
        Ok(sqlx::query_as::<_, (String, i64)>(
            "SELECT event_type::text, count(*) FROM backlink_events \
             WHERE organization_id = $1 AND detected_at >= $2 GROUP BY event_type",
        )
        .bind(scope.organization_id())
        .bind(since)
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
             WHERE organization_id = $1 AND archived_at IS NULL AND subject = 'own' AND tier = 1 AND cost IS NOT NULL \
               AND status IN ('lost','suspected_lost','changed','nofollow','source_error','target_error','redirected') \
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
             WHERE organization_id = $1 AND archived_at IS NULL AND subject = 'own' AND tier = 1 AND cost IS NOT NULL \
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

    /// Open a 10-day, no-card free trial for a new workspace (docs/20). The trial
    /// entitlement is Starter's limits (500 backlinks / 3 projects / 1 seat).
    /// No Stripe customer yet, so a placeholder id satisfies the NOT NULL column.
    /// Idempotent: a workspace that already has a subscription is left untouched.
    pub async fn start_trial(pool: &PgPool, organization_id: Uuid) -> Result<(), DbError> {
        sqlx::query(
            "INSERT INTO subscriptions \
                (id, organization_id, stripe_customer_id, plan, status, \
                 current_period_start, current_period_end, trial_end) \
             VALUES ($1, $2, $3, 'starter'::subscription_plan, 'trialing', \
                 now(), now() + interval '10 days', now() + interval '10 days') \
             ON CONFLICT (organization_id) DO NOTHING",
        )
        .bind(Uuid::now_v7())
        .bind(organization_id)
        .bind(format!("trial:{organization_id}"))
        .execute(pool)
        .await
        .map_err(DbError::from_insert)?;
        Ok(())
    }

    /// Whether the org may use paid functionality right now: an `active`/`past_due`
    /// subscription, or a `trialing` one whose `trial_end` is still in the future.
    /// An expired trial or canceled subscription returns false — new adds and
    /// monitoring are blocked, but existing data is never touched (docs/20).
    pub async fn is_access_active(pool: &PgPool, organization_id: Uuid) -> Result<bool, DbError> {
        let (ok,): (bool,) = sqlx::query_as(
            "SELECT EXISTS ( \
                SELECT 1 FROM subscriptions \
                WHERE organization_id = $1 \
                  AND ( status IN ('active','past_due') \
                     OR (status = 'trialing' AND trial_end > now()) ) )",
        )
        .bind(organization_id)
        .fetch_one(pool)
        .await?;
        Ok(ok)
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

    /// The org's Stripe customer id, if it has a subscription row (for the Portal).
    pub async fn customer_for_org(
        pool: &PgPool,
        organization_id: Uuid,
    ) -> Result<Option<String>, DbError> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT stripe_customer_id FROM subscriptions WHERE organization_id = $1",
        )
        .bind(organization_id)
        .fetch_optional(pool)
        .await?;
        Ok(row.map(|(c,)| c))
    }

    /// Plan + status + trial end for display (defaults to starter / none / no
    /// trial when unsubscribed). `trial_end` is the raw timestamp; the API layer
    /// derives "days left" from it so we never decode a Postgres interval here.
    pub async fn status_for_org(
        pool: &PgPool,
        organization_id: Uuid,
    ) -> Result<(String, String, Option<chrono::DateTime<chrono::Utc>>), DbError> {
        let row: Option<(String, String, Option<chrono::DateTime<chrono::Utc>>)> = sqlx::query_as(
            "SELECT plan::text, status, trial_end FROM subscriptions WHERE organization_id = $1",
        )
        .bind(organization_id)
        .fetch_optional(pool)
        .await?;
        Ok(row.unwrap_or_else(|| ("starter".to_string(), "none".to_string(), None)))
    }

    /// Upsert the org's subscription from a completed checkout (webhook). One row
    /// per org (organization_id is UNIQUE); a re-subscribe updates in place.
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_subscription(
        pool: &PgPool,
        organization_id: Uuid,
        customer_id: &str,
        subscription_id: Option<&str>,
        plan_slug: &str,
        status: &str,
    ) -> Result<(), DbError> {
        sqlx::query(
            "INSERT INTO subscriptions \
                (id, organization_id, stripe_customer_id, stripe_subscription_id, plan, status) \
             VALUES ($1, $2, $3, $4, $5::subscription_plan, $6) \
             ON CONFLICT (organization_id) DO UPDATE SET \
                stripe_customer_id = EXCLUDED.stripe_customer_id, \
                stripe_subscription_id = EXCLUDED.stripe_subscription_id, \
                plan = EXCLUDED.plan, status = EXCLUDED.status, updated_at = now()",
        )
        .bind(Uuid::now_v7())
        .bind(organization_id)
        .bind(customer_id)
        .bind(subscription_id)
        .bind(plan_slug)
        .bind(status)
        .execute(pool)
        .await
        .map_err(DbError::from_insert)?;
        Ok(())
    }

    /// Update plan/status by Stripe subscription id (subscription.deleted, and
    /// the invoice fallbacks). COALESCE keeps the existing plan when the event
    /// carries no price we map. When the new status is `canceled` we stamp
    /// `canceled_at` and clear the pending cancel flag.
    pub async fn update_subscription_by_stripe_id(
        pool: &PgPool,
        subscription_id: &str,
        plan_slug: Option<&str>,
        status: &str,
    ) -> Result<(), DbError> {
        sqlx::query(
            "UPDATE subscriptions SET \
                status = $2, \
                plan = COALESCE($3::subscription_plan, plan), \
                canceled_at = CASE WHEN $2 = 'canceled' THEN now() ELSE canceled_at END, \
                cancel_at_period_end = CASE WHEN $2 = 'canceled' THEN false ELSE cancel_at_period_end END, \
                updated_at = now() \
             WHERE stripe_subscription_id = $1",
        )
        .bind(subscription_id)
        .bind(status)
        .bind(plan_slug)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Full subscription upsert from a `customer.subscription.*` event (docs/20 —
    /// P4). Keyed by organization (one row per org), so it is robust to event
    /// ordering: it creates the row if `checkout.session.completed` has not landed
    /// yet, otherwise updates it in place. Period/trial fields use COALESCE so a
    /// later event that omits one keeps the stored value instead of nulling it.
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_subscription_from_stripe(
        pool: &PgPool,
        organization_id: Uuid,
        customer_id: &str,
        subscription_id: Option<&str>,
        plan_slug: Option<&str>,
        status: &str,
        current_period_start: Option<chrono::DateTime<chrono::Utc>>,
        current_period_end: Option<chrono::DateTime<chrono::Utc>>,
        cancel_at_period_end: bool,
        canceled_at: Option<chrono::DateTime<chrono::Utc>>,
        trial_end: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<(), DbError> {
        sqlx::query(
            "INSERT INTO subscriptions \
                (id, organization_id, stripe_customer_id, stripe_subscription_id, plan, status, \
                 current_period_start, current_period_end, cancel_at_period_end, \
                 canceled_at, trial_end) \
             VALUES ($1, $2, $3, $4, \
                     COALESCE($5::subscription_plan, 'starter'::subscription_plan), $6, \
                     $7, $8, $9, $10, $11) \
             ON CONFLICT (organization_id) DO UPDATE SET \
                stripe_customer_id = EXCLUDED.stripe_customer_id, \
                stripe_subscription_id = \
                    COALESCE(EXCLUDED.stripe_subscription_id, subscriptions.stripe_subscription_id), \
                plan = COALESCE($5::subscription_plan, subscriptions.plan), \
                status = EXCLUDED.status, \
                current_period_start = \
                    COALESCE(EXCLUDED.current_period_start, subscriptions.current_period_start), \
                current_period_end = \
                    COALESCE(EXCLUDED.current_period_end, subscriptions.current_period_end), \
                cancel_at_period_end = EXCLUDED.cancel_at_period_end, \
                canceled_at = EXCLUDED.canceled_at, \
                trial_end = COALESCE(EXCLUDED.trial_end, subscriptions.trial_end), \
                updated_at = now()",
        )
        .bind(Uuid::now_v7())
        .bind(organization_id)
        .bind(customer_id)
        .bind(subscription_id)
        .bind(plan_slug)
        .bind(status)
        .bind(current_period_start)
        .bind(current_period_end)
        .bind(cancel_at_period_end)
        .bind(canceled_at)
        .bind(trial_end)
        .execute(pool)
        .await
        .map_err(DbError::from_insert)?;
        Ok(())
    }

    /// Dunning: a failed invoice puts a live subscription into `past_due` (the
    /// grace state — `is_access_active` still grants access). Never resurrects a
    /// canceled/expired subscription.
    pub async fn mark_past_due(pool: &PgPool, subscription_id: &str) -> Result<(), DbError> {
        sqlx::query(
            "UPDATE subscriptions SET status = 'past_due', updated_at = now() \
             WHERE stripe_subscription_id = $1 AND status IN ('active', 'trialing', 'past_due')",
        )
        .bind(subscription_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Dunning recovery: a succeeded invoice brings a `past_due` subscription back
    /// to `active`. Only touches `past_due` rows, so it never overrides a trial or
    /// a cancellation.
    pub async fn mark_active_if_recoverable(
        pool: &PgPool,
        subscription_id: &str,
    ) -> Result<(), DbError> {
        sqlx::query(
            "UPDATE subscriptions SET status = 'active', updated_at = now() \
             WHERE stripe_subscription_id = $1 AND status = 'past_due'",
        )
        .bind(subscription_id)
        .execute(pool)
        .await?;
        Ok(())
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

    /// Partial edit of a campaign (§22): any subset of name/description/status/
    /// target/budget+currency. `NotFound` if not in the org.
    #[allow(clippy::too_many_arguments)]
    pub async fn update(
        pool: &PgPool,
        scope: &OrgScope,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        status: Option<&str>,
        target_backlinks: Option<i32>,
        budget: Option<&str>,
        currency: Option<&str>,
    ) -> Result<Campaign, DbError> {
        let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new("UPDATE campaigns SET ");
        let mut sep = qb.separated(", ");
        if let Some(n) = name {
            sep.push("name = ");
            sep.push_bind_unseparated(n);
        }
        if let Some(d) = description {
            sep.push("description = ");
            sep.push_bind_unseparated(d);
        }
        if let Some(s) = status {
            sep.push("status = ");
            sep.push_bind_unseparated(s.to_string());
            sep.push_unseparated("::campaign_status");
        }
        if let Some(t) = target_backlinks {
            sep.push("target_backlinks = ");
            sep.push_bind_unseparated(t);
        }
        if let Some(b) = budget {
            sep.push("budget = ");
            sep.push_bind_unseparated(b.to_string());
            sep.push_unseparated("::numeric");
        }
        if let Some(c) = currency {
            sep.push("currency = ");
            sep.push_bind_unseparated(c);
        }
        qb.push(" WHERE id = ");
        qb.push_bind(id);
        qb.push(" AND organization_id = ");
        qb.push_bind(scope.organization_id());
        qb.push(concat!(" RETURNING ", cols!()));
        qb.build_query_as::<Campaign>()
            .fetch_one(pool)
            .await
            .map_err(DbError::from_fetch)
    }

    /// Delete a campaign (§22). Attached backlinks are unassigned first — the
    /// composite FK's plain `SET NULL` would also try to null the NOT NULL
    /// `organization_id`, so we clear `campaign_id` ourselves in one
    /// transaction. `NotFound` if not in the org.
    pub async fn delete(pool: &PgPool, scope: &OrgScope, id: Uuid) -> Result<(), DbError> {
        let mut tx = pool.begin().await?;
        sqlx::query(
            "UPDATE backlinks SET campaign_id = NULL \
             WHERE campaign_id = $1 AND organization_id = $2",
        )
        .bind(id)
        .bind(scope.organization_id())
        .execute(&mut *tx)
        .await?;
        let row: Option<(Uuid,)> = sqlx::query_as(
            "DELETE FROM campaigns WHERE id = $1 AND organization_id = $2 RETURNING id",
        )
        .bind(id)
        .bind(scope.organization_id())
        .fetch_optional(&mut *tx)
        .await?;
        if row.is_none() {
            return Err(DbError::NotFound); // tx rolls back on drop
        }
        tx.commit().await?;
        Ok(())
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

    /// Delete a publisher (§31). Attached backlinks are unassigned first (same
    /// composite-FK reason as campaigns). `NotFound` if not in the org.
    pub async fn delete(pool: &PgPool, scope: &OrgScope, id: Uuid) -> Result<(), DbError> {
        let mut tx = pool.begin().await?;
        sqlx::query(
            "UPDATE backlinks SET publisher_id = NULL \
             WHERE publisher_id = $1 AND organization_id = $2",
        )
        .bind(id)
        .bind(scope.organization_id())
        .execute(&mut *tx)
        .await?;
        let row: Option<(Uuid,)> = sqlx::query_as(
            "DELETE FROM publishers WHERE id = $1 AND organization_id = $2 RETURNING id",
        )
        .bind(id)
        .bind(scope.organization_id())
        .fetch_optional(&mut *tx)
        .await?;
        if row.is_none() {
            return Err(DbError::NotFound);
        }
        tx.commit().await?;
        Ok(())
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

    /// The org's notifications feed, newest first (§37). Tenant-scoped.
    pub async fn list(
        pool: &PgPool,
        scope: &OrgScope,
        limit: i64,
    ) -> Result<Vec<crate::entities::Notification>, DbError> {
        Ok(sqlx::query_as::<_, crate::entities::Notification>(
            "SELECT id, kind, title, body, backlink_id, read_at, created_at \
             FROM notifications WHERE organization_id = $1 \
             ORDER BY created_at DESC LIMIT $2",
        )
        .bind(scope.organization_id())
        .bind(limit.clamp(1, 200))
        .fetch_all(pool)
        .await?)
    }

    /// Mark one notification read (idempotent). Scoped to the org, so a
    /// cross-tenant id yields `NotFound`, never another tenant's row. Marking an
    /// already-read one is a no-op success.
    pub async fn mark_read(pool: &PgPool, scope: &OrgScope, id: Uuid) -> Result<(), DbError> {
        let row: Option<(Uuid,)> = sqlx::query_as(
            "UPDATE notifications SET read_at = coalesce(read_at, now()) \
             WHERE id = $1 AND organization_id = $2 RETURNING id",
        )
        .bind(id)
        .bind(scope.organization_id())
        .fetch_optional(pool)
        .await?;
        match row {
            Some(_) => Ok(()),
            None => Err(DbError::NotFound),
        }
    }
}

// ─── Alert destinations + delivery ledger (§37, migration 0024) ──────────────
pub mod alerts {
    use super::*;
    use crate::entities::AlertDestination;

    pub async fn create(
        pool: &PgPool,
        scope: &OrgScope,
        id: Uuid,
        kind: &str,
        target: &str,
        name: &str,
        digest: &str,
    ) -> Result<AlertDestination, DbError> {
        sqlx::query_as::<_, AlertDestination>(
            "INSERT INTO alert_destinations (id, organization_id, kind, target, name, digest) \
             VALUES ($1, $2, $3, $4, $5, $6) \
             RETURNING id, organization_id, kind, target, name, enabled, digest, \
                       last_status, last_error, last_delivered_at, created_at",
        )
        .bind(id)
        .bind(scope.organization_id())
        .bind(kind)
        .bind(target)
        .bind(name)
        .bind(digest)
        .fetch_one(pool)
        .await
        .map_err(DbError::from_insert)
    }

    pub async fn list(pool: &PgPool, scope: &OrgScope) -> Result<Vec<AlertDestination>, DbError> {
        Ok(sqlx::query_as::<_, AlertDestination>(
            "SELECT id, organization_id, kind, target, name, enabled, digest, \
                    last_status, last_error, last_delivered_at, created_at \
             FROM alert_destinations WHERE organization_id = $1 ORDER BY created_at",
        )
        .bind(scope.organization_id())
        .fetch_all(pool)
        .await?)
    }

    pub async fn get(
        pool: &PgPool,
        scope: &OrgScope,
        id: Uuid,
    ) -> Result<AlertDestination, DbError> {
        sqlx::query_as::<_, AlertDestination>(
            "SELECT id, organization_id, kind, target, name, enabled, digest, \
                    last_status, last_error, last_delivered_at, created_at \
             FROM alert_destinations WHERE id = $1 AND organization_id = $2",
        )
        .bind(id)
        .bind(scope.organization_id())
        .fetch_one(pool)
        .await
        .map_err(DbError::from_fetch)
    }

    pub async fn set_enabled(
        pool: &PgPool,
        scope: &OrgScope,
        id: Uuid,
        enabled: bool,
    ) -> Result<AlertDestination, DbError> {
        sqlx::query_as::<_, AlertDestination>(
            "UPDATE alert_destinations SET enabled = $3 \
             WHERE id = $1 AND organization_id = $2 \
             RETURNING id, organization_id, kind, target, name, enabled, digest, \
                       last_status, last_error, last_delivered_at, created_at",
        )
        .bind(id)
        .bind(scope.organization_id())
        .bind(enabled)
        .fetch_one(pool)
        .await
        .map_err(DbError::from_fetch)
    }

    pub async fn set_digest(
        pool: &PgPool,
        scope: &OrgScope,
        id: Uuid,
        digest: &str,
    ) -> Result<AlertDestination, DbError> {
        sqlx::query_as::<_, AlertDestination>(
            "UPDATE alert_destinations SET digest = $3 \
             WHERE id = $1 AND organization_id = $2 \
             RETURNING id, organization_id, kind, target, name, enabled, digest, \
                       last_status, last_error, last_delivered_at, created_at",
        )
        .bind(id)
        .bind(scope.organization_id())
        .bind(digest)
        .fetch_one(pool)
        .await
        .map_err(DbError::from_fetch)
    }

    pub async fn delete(pool: &PgPool, scope: &OrgScope, id: Uuid) -> Result<(), DbError> {
        let row: Option<(Uuid,)> = sqlx::query_as(
            "DELETE FROM alert_destinations WHERE id = $1 AND organization_id = $2 RETURNING id",
        )
        .bind(id)
        .bind(scope.organization_id())
        .fetch_optional(pool)
        .await?;
        row.map(|_| ()).ok_or(DbError::NotFound)
    }

    /// Record the last outbound attempt on a destination (not tenant-scoped: the
    /// delivery tick already holds the destination row).
    pub async fn set_status(
        pool: &PgPool,
        id: Uuid,
        status: &str,
        error: Option<&str>,
    ) -> Result<(), DbError> {
        sqlx::query(
            "UPDATE alert_destinations \
             SET last_status = $2, last_error = $3, \
                 last_delivered_at = CASE WHEN $2 = 'ok' THEN now() ELSE last_delivered_at END \
             WHERE id = $1",
        )
        .bind(id)
        .bind(status)
        .bind(error)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// All enabled destinations across every org — the delivery tick iterates
    /// these. Bounded so a huge instance can't build an unbounded worklist.
    pub async fn list_all_active(pool: &PgPool) -> Result<Vec<AlertDestination>, DbError> {
        Ok(sqlx::query_as::<_, AlertDestination>(
            "SELECT id, organization_id, kind, target, name, enabled, digest, \
                    last_status, last_error, last_delivered_at, created_at \
             FROM alert_destinations WHERE enabled ORDER BY created_at LIMIT 500",
        )
        .fetch_all(pool)
        .await?)
    }

    /// A notification queued for a destination: recent, and not yet delivered
    /// there (no ledger row).
    #[derive(Debug, Clone, sqlx::FromRow)]
    pub struct PendingNotification {
        pub id: Uuid,
        pub kind: String,
        pub title: String,
        pub body: Option<String>,
        pub created_at: DateTime<Utc>,
    }

    pub async fn pending_for_destination(
        pool: &PgPool,
        destination_id: Uuid,
        organization_id: Uuid,
        limit: i64,
    ) -> Result<Vec<PendingNotification>, DbError> {
        Ok(sqlx::query_as::<_, PendingNotification>(
            "SELECT n.id, n.kind, n.title, n.body, n.created_at \
             FROM notifications n \
             WHERE n.organization_id = $2 \
               AND n.created_at > now() - interval '24 hours' \
               AND NOT EXISTS ( \
                 SELECT 1 FROM alert_deliveries d \
                 WHERE d.destination_id = $1 AND d.notification_id = n.id) \
             ORDER BY n.created_at \
             LIMIT $3",
        )
        .bind(destination_id)
        .bind(organization_id)
        .bind(limit.clamp(1, 100))
        .fetch_all(pool)
        .await?)
    }

    /// Append a delivery ledger row (idempotent on the unique constraint).
    pub async fn record_delivery(
        pool: &PgPool,
        destination_id: Uuid,
        notification_id: Uuid,
        status: &str,
        http_status: Option<i32>,
        error: Option<&str>,
    ) -> Result<(), DbError> {
        sqlx::query(
            "INSERT INTO alert_deliveries \
               (id, destination_id, notification_id, status, http_status, error) \
             VALUES ($1, $2, $3, $4, $5, $6) \
             ON CONFLICT (destination_id, notification_id) DO NOTHING",
        )
        .bind(Uuid::now_v7())
        .bind(destination_id)
        .bind(notification_id)
        .bind(status)
        .bind(http_status)
        .bind(error)
        .execute(pool)
        .await?;
        Ok(())
    }
}

// ─── Tags (§35) — tenant-scoped, with backlink tagging ───────────────────────
pub mod tags {
    use super::*;
    use crate::entities::Tag;

    // name is citext; cast to text so no custom sqlx type mapping is needed.
    macro_rules! cols {
        () => {
            "id, organization_id, name::text AS name, color, created_at"
        };
    }

    pub async fn create(
        pool: &PgPool,
        scope: &OrgScope,
        id: Uuid,
        name: &str,
        color: Option<&str>,
    ) -> Result<Tag, DbError> {
        sqlx::query_as::<_, Tag>(concat!(
            "INSERT INTO tags (id, organization_id, name, color) \
             VALUES ($1, $2, $3, $4) RETURNING ",
            cols!()
        ))
        .bind(id)
        .bind(scope.organization_id())
        .bind(name)
        .bind(color)
        .fetch_one(pool)
        .await
        .map_err(DbError::from_insert)
    }

    pub async fn list(pool: &PgPool, scope: &OrgScope) -> Result<Vec<Tag>, DbError> {
        Ok(sqlx::query_as::<_, Tag>(concat!(
            "SELECT ",
            cols!(),
            " FROM tags WHERE organization_id = $1 ORDER BY name"
        ))
        .bind(scope.organization_id())
        .fetch_all(pool)
        .await?)
    }

    pub async fn get(pool: &PgPool, scope: &OrgScope, id: Uuid) -> Result<Tag, DbError> {
        sqlx::query_as::<_, Tag>(concat!(
            "SELECT ",
            cols!(),
            " FROM tags WHERE id = $1 AND organization_id = $2"
        ))
        .bind(id)
        .bind(scope.organization_id())
        .fetch_one(pool)
        .await
        .map_err(DbError::from_fetch)
    }

    /// Delete a tag (§35). Its attachments are removed (`backlink_tags.tag_id`
    /// FK is `ON DELETE CASCADE`). `NotFound` if not in the org.
    pub async fn delete(pool: &PgPool, scope: &OrgScope, id: Uuid) -> Result<(), DbError> {
        let row: Option<(Uuid,)> =
            sqlx::query_as("DELETE FROM tags WHERE id = $1 AND organization_id = $2 RETURNING id")
                .bind(id)
                .bind(scope.organization_id())
                .fetch_optional(pool)
                .await?;
        row.map(|_| ()).ok_or(DbError::NotFound)
    }

    /// One (backlink_id, tag) pairing — for listing tags across a page of
    /// backlinks in a single query.
    #[derive(Debug, Clone, sqlx::FromRow)]
    pub struct BacklinkTagRow {
        pub backlink_id: Uuid,
        pub id: Uuid,
        pub name: String,
        pub color: Option<String>,
    }

    /// Tags for many backlinks at once (the list view's Tags column). Tenant-scoped.
    pub async fn for_backlinks(
        pool: &PgPool,
        scope: &OrgScope,
        ids: &[Uuid],
    ) -> Result<Vec<BacklinkTagRow>, DbError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        Ok(sqlx::query_as::<_, BacklinkTagRow>(
            "SELECT bt.backlink_id, t.id, t.name::text AS name, t.color \
             FROM tags t JOIN backlink_tags bt ON bt.tag_id = t.id \
             WHERE bt.organization_id = $1 AND bt.backlink_id = ANY($2) \
             ORDER BY t.name",
        )
        .bind(scope.organization_id())
        .bind(ids)
        .fetch_all(pool)
        .await?)
    }

    /// The tags attached to one backlink, alphabetical. Tenant-scoped via the
    /// backlink_tags.organization_id column.
    pub async fn for_backlink(
        pool: &PgPool,
        scope: &OrgScope,
        backlink_id: Uuid,
    ) -> Result<Vec<Tag>, DbError> {
        Ok(sqlx::query_as::<_, Tag>(
            "SELECT t.id, t.organization_id, t.name::text AS name, t.color, t.created_at \
             FROM tags t JOIN backlink_tags bt ON bt.tag_id = t.id \
             WHERE bt.organization_id = $1 AND bt.backlink_id = $2 \
             ORDER BY t.name",
        )
        .bind(scope.organization_id())
        .bind(backlink_id)
        .fetch_all(pool)
        .await?)
    }

    /// Attach a tag to a backlink, idempotently. Both ids are validated to be in
    /// the org by the caller; the composite FKs are a second guard. Returns true
    /// if the link was newly created.
    pub async fn attach(
        pool: &PgPool,
        scope: &OrgScope,
        backlink_id: Uuid,
        tag_id: Uuid,
    ) -> Result<bool, DbError> {
        let row: Option<(Uuid,)> = sqlx::query_as(
            "INSERT INTO backlink_tags (organization_id, backlink_id, tag_id) \
             VALUES ($1, $2, $3) ON CONFLICT (backlink_id, tag_id) DO NOTHING \
             RETURNING backlink_id",
        )
        .bind(scope.organization_id())
        .bind(backlink_id)
        .bind(tag_id)
        .fetch_optional(pool)
        .await
        .map_err(DbError::from_insert)?;
        Ok(row.is_some())
    }

    /// Detach a tag from a backlink. `NotFound` if the pairing did not exist.
    pub async fn detach(
        pool: &PgPool,
        scope: &OrgScope,
        backlink_id: Uuid,
        tag_id: Uuid,
    ) -> Result<(), DbError> {
        let n = sqlx::query(
            "DELETE FROM backlink_tags \
             WHERE organization_id = $1 AND backlink_id = $2 AND tag_id = $3",
        )
        .bind(scope.organization_id())
        .bind(backlink_id)
        .bind(tag_id)
        .execute(pool)
        .await?
        .rows_affected();
        if n == 0 {
            Err(DbError::NotFound)
        } else {
            Ok(())
        }
    }
}

// ─── Members (team) — the org roster + role changes ──────────────────────────
pub mod members {
    use super::*;
    use crate::entities::Member;

    /// Seat count (every member; the owner counts as a seat). Takes a raw org id
    /// because it is also used at invite-accept time, before membership exists.
    pub async fn count_for_org(pool: &PgPool, organization_id: Uuid) -> Result<i64, DbError> {
        let (n,): (i64,) =
            sqlx::query_as("SELECT count(*) FROM organization_members WHERE organization_id = $1")
                .bind(organization_id)
                .fetch_one(pool)
                .await?;
        Ok(n)
    }

    /// Everyone in the org with their role, owners first then by join time.
    pub async fn list(pool: &PgPool, scope: &OrgScope) -> Result<Vec<Member>, DbError> {
        Ok(sqlx::query_as::<_, Member>(
            "SELECT u.id AS user_id, u.email::text AS email, u.name, \
                    m.role::text AS role, m.created_at \
             FROM organization_members m JOIN users u ON u.id = m.user_id \
             WHERE m.organization_id = $1 \
             ORDER BY (m.role = 'owner') DESC, m.created_at",
        )
        .bind(scope.organization_id())
        .fetch_all(pool)
        .await?)
    }

    /// Add a user to the org with a role, idempotently (keeps the existing role
    /// if already a member). Used when an invitation is accepted.
    pub async fn add(
        pool: &PgPool,
        organization_id: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> Result<(), DbError> {
        sqlx::query(
            "INSERT INTO organization_members (organization_id, user_id, role) \
             VALUES ($1, $2, $3::org_role) \
             ON CONFLICT (organization_id, user_id) DO NOTHING",
        )
        .bind(organization_id)
        .bind(user_id)
        .bind(role)
        .execute(pool)
        .await
        .map_err(DbError::from_insert)?;
        Ok(())
    }
}

// ─── Invitations (team) ──────────────────────────────────────────────────────
pub mod invitations {
    use super::*;
    use crate::entities::Invitation;

    macro_rules! cols {
        () => {
            "id, organization_id, email::text AS email, role::text AS role, \
             invited_by, expires_at, accepted_at, created_at"
        };
    }

    /// Fields for a new invitation (keeps `create` to a few arguments).
    pub struct NewInvitation<'a> {
        pub email: &'a str,
        pub role: &'a str,
        pub invited_by: Uuid,
        pub token_hash: &'a [u8],
        pub expires_at: DateTime<Utc>,
    }

    pub async fn create(
        pool: &PgPool,
        scope: &OrgScope,
        id: Uuid,
        new: NewInvitation<'_>,
    ) -> Result<Invitation, DbError> {
        sqlx::query_as::<_, Invitation>(concat!(
            "INSERT INTO organization_invitations \
                (id, organization_id, email, role, invited_by, token_hash, expires_at) \
             VALUES ($1, $2, $3, $4::org_role, $5, $6, $7) RETURNING ",
            cols!()
        ))
        .bind(id)
        .bind(scope.organization_id())
        .bind(new.email)
        .bind(new.role)
        .bind(new.invited_by)
        .bind(new.token_hash)
        .bind(new.expires_at)
        .fetch_one(pool)
        .await
        .map_err(DbError::from_insert)
    }

    /// Count of outstanding (unaccepted) invitations — committed future seats.
    pub async fn count_pending_for_org(
        pool: &PgPool,
        organization_id: Uuid,
    ) -> Result<i64, DbError> {
        let (n,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM organization_invitations \
             WHERE organization_id = $1 AND accepted_at IS NULL",
        )
        .bind(organization_id)
        .fetch_one(pool)
        .await?;
        Ok(n)
    }

    /// Pending (unaccepted) invitations for the org, newest first.
    pub async fn list_pending(pool: &PgPool, scope: &OrgScope) -> Result<Vec<Invitation>, DbError> {
        Ok(sqlx::query_as::<_, Invitation>(concat!(
            "SELECT ",
            cols!(),
            " FROM organization_invitations \
              WHERE organization_id = $1 AND accepted_at IS NULL \
              ORDER BY created_at DESC"
        ))
        .bind(scope.organization_id())
        .fetch_all(pool)
        .await?)
    }

    /// Revoke a pending invitation. `NotFound` if it is missing, already
    /// accepted, or belongs to another org.
    pub async fn revoke(pool: &PgPool, scope: &OrgScope, id: Uuid) -> Result<(), DbError> {
        let row: Option<(Uuid,)> = sqlx::query_as(
            "DELETE FROM organization_invitations \
             WHERE id = $1 AND organization_id = $2 AND accepted_at IS NULL RETURNING id",
        )
        .bind(id)
        .bind(scope.organization_id())
        .fetch_optional(pool)
        .await?;
        row.map(|_| ()).ok_or(DbError::NotFound)
    }

    /// Look up a live invitation by its token hash — a **system** query (the
    /// accepting user is not yet a member). Only returns one that is unaccepted
    /// and unexpired.
    pub async fn find_live_by_token_hash(
        pool: &PgPool,
        token_hash: &[u8],
        now: DateTime<Utc>,
    ) -> Result<Invitation, DbError> {
        sqlx::query_as::<_, Invitation>(concat!(
            "SELECT ",
            cols!(),
            " FROM organization_invitations \
              WHERE token_hash = $1 AND accepted_at IS NULL AND expires_at > $2"
        ))
        .bind(token_hash)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(DbError::from_fetch)
    }

    /// Accept an invitation and add the user to the org, atomically. Marks the
    /// invitation accepted and inserts the membership in one transaction.
    pub async fn accept(
        pool: &PgPool,
        invitation_id: Uuid,
        organization_id: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> Result<(), DbError> {
        let mut tx = pool.begin().await?;
        sqlx::query(
            "INSERT INTO organization_members (organization_id, user_id, role) \
             VALUES ($1, $2, $3::org_role) \
             ON CONFLICT (organization_id, user_id) DO NOTHING",
        )
        .bind(organization_id)
        .bind(user_id)
        .bind(role)
        .execute(&mut *tx)
        .await
        .map_err(DbError::from_insert)?;
        sqlx::query("UPDATE organization_invitations SET accepted_at = now() WHERE id = $1")
            .bind(invitation_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
}

// ─── Notification preferences (§37) ──────────────────────────────────────────
pub mod notification_prefs {
    use super::*;
    use crate::entities::NotificationPreference;

    /// The user's stored preferences for one org. A missing row means the
    /// default (enabled, immediate) for that event/channel.
    pub async fn list(
        pool: &PgPool,
        user_id: Uuid,
        organization_id: Uuid,
    ) -> Result<Vec<NotificationPreference>, DbError> {
        Ok(sqlx::query_as::<_, NotificationPreference>(
            "SELECT event_type::text AS event_type, channel, enabled, digest \
             FROM notification_preferences \
             WHERE user_id = $1 AND organization_id = $2 \
             ORDER BY event_type, channel",
        )
        .bind(user_id)
        .bind(organization_id)
        .fetch_all(pool)
        .await?)
    }

    /// Upsert one preference (unique on user+org+event+channel).
    pub async fn upsert(
        pool: &PgPool,
        user_id: Uuid,
        organization_id: Uuid,
        event_type: &str,
        channel: &str,
        enabled: bool,
        digest: &str,
    ) -> Result<(), DbError> {
        sqlx::query(
            "INSERT INTO notification_preferences \
                (id, user_id, organization_id, event_type, channel, enabled, digest) \
             VALUES ($1, $2, $3, $4::event_type, $5, $6, $7) \
             ON CONFLICT (user_id, organization_id, event_type, channel) \
             DO UPDATE SET enabled = EXCLUDED.enabled, digest = EXCLUDED.digest",
        )
        .bind(Uuid::now_v7())
        .bind(user_id)
        .bind(organization_id)
        .bind(event_type)
        .bind(channel)
        .bind(enabled)
        .bind(digest)
        .execute(pool)
        .await
        .map_err(DbError::from_insert)?;
        Ok(())
    }
}
