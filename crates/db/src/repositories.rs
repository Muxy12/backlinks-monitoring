//! Repositories for the identity/tenancy slice (docs/03, docs/05). Tenant-owned
//! reads take an [`OrgScope`](crate::scope::OrgScope) so a query cannot forget
//! its tenant filter (ADR-0004). The identity tables (`users`, `sessions`) are
//! not tenant-owned and are keyed by their own ids.
//!
//! All queries are runtime (`sqlx::query_as`), so they need no compile-time
//! database or `.sqlx` cache.

use crate::entities::{Organization, Session, User};
use crate::error::DbError;
use crate::scope::OrgScope;
use chrono::{DateTime, Utc};
use common::Role;
use sqlx::PgPool;
use uuid::Uuid;

const USER_COLS: &str = "id, email, password_hash, name, email_verified_at, created_at, updated_at";
const ORG_COLS: &str = "id, name, slug, created_at, updated_at";
const SESSION_COLS: &str = "id, user_id, token_hash, csrf_token_hash, expires_at, revoked_at";

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
        let sql = format!(
            "INSERT INTO users (id, email, password_hash, name) \
             VALUES ($1, $2, $3, $4) RETURNING {USER_COLS}"
        );
        sqlx::query_as::<_, User>(&sql)
            .bind(id)
            .bind(email)
            .bind(password_hash)
            .bind(name)
            .fetch_one(pool)
            .await
            .map_err(DbError::from_insert)
    }

    pub async fn find_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, DbError> {
        let sql = format!("SELECT {USER_COLS} FROM users WHERE email = $1");
        Ok(sqlx::query_as::<_, User>(&sql)
            .bind(email)
            .fetch_optional(pool)
            .await?)
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<User, DbError> {
        let sql = format!("SELECT {USER_COLS} FROM users WHERE id = $1");
        sqlx::query_as::<_, User>(&sql)
            .bind(id)
            .fetch_one(pool)
            .await
            .map_err(DbError::from_fetch)
    }

    pub async fn mark_email_verified(pool: &PgPool, id: Uuid) -> Result<(), DbError> {
        sqlx::query("UPDATE users SET email_verified_at = now() WHERE id = $1 AND email_verified_at IS NULL")
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

        let sql = format!(
            "INSERT INTO organizations (id, name, slug) VALUES ($1, $2, $3) RETURNING {ORG_COLS}"
        );
        let org = sqlx::query_as::<_, Organization>(&sql)
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

    /// Fetch an org the caller is scoped to. Scoped by the verified
    /// `OrgScope`, so this cannot return another tenant's row.
    pub async fn get(pool: &PgPool, scope: &OrgScope) -> Result<Organization, DbError> {
        let sql = format!("SELECT {ORG_COLS} FROM organizations WHERE id = $1");
        sqlx::query_as::<_, Organization>(&sql)
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
        let sql = format!(
            "SELECT {}, m.role::text AS role \
             FROM organizations o \
             JOIN organization_members m ON m.organization_id = o.id \
             WHERE m.user_id = $1 \
             ORDER BY o.created_at",
            ORG_COLS
                .split(", ")
                .map(|c| format!("o.{c}"))
                .collect::<Vec<_>>()
                .join(", ")
        );
        let rows =
            sqlx::query_as::<_, (Uuid, String, String, DateTime<Utc>, DateTime<Utc>, String)>(&sql)
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

    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        pool: &PgPool,
        id: Uuid,
        user_id: Uuid,
        token_hash: &[u8],
        csrf_token_hash: &[u8],
        expires_at: DateTime<Utc>,
    ) -> Result<Session, DbError> {
        let sql = format!(
            "INSERT INTO sessions (id, user_id, token_hash, csrf_token_hash, expires_at) \
             VALUES ($1, $2, $3, $4, $5) RETURNING {SESSION_COLS}"
        );
        sqlx::query_as::<_, Session>(&sql)
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
        let sql = format!(
            "SELECT {SESSION_COLS} FROM sessions \
             WHERE token_hash = $1 AND revoked_at IS NULL AND expires_at > $2"
        );
        sqlx::query_as::<_, Session>(&sql)
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
