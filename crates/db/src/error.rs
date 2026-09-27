//! Database error type. Thin wrapper over `sqlx::Error` plus the couple of
//! domain-shaped variants callers care about.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("migration error: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),

    /// A fetch-by-id found nothing in the caller's tenant scope. Repositories
    /// map this to a 404 (never 403) so cross-tenant access is indistinguishable
    /// from non-existence (docs/05, ADR-0004).
    #[error("not found")]
    NotFound,

    /// The actor is a member but lacks the required role (docs/05 §2). Distinct
    /// from `NotFound`: the resource is in scope, the action is not permitted.
    #[error("forbidden")]
    Forbidden,

    /// The organization has been suspended by a platform admin (migration 0023).
    /// A member is refused a scope until it is unsuspended.
    #[error("organization suspended")]
    Suspended,

    /// A uniqueness constraint was violated (e.g. duplicate email or slug).
    /// Carries the constraint name so callers can map it to a field error.
    #[error("conflict: {0}")]
    Conflict(String),

    #[error("invalid database configuration: {0}")]
    Config(String),
}

impl DbError {
    /// Convenience: turn a `RowNotFound` into our `NotFound`, leaving other
    /// errors intact. Repositories use this on `fetch_one`.
    pub fn from_fetch(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::RowNotFound => DbError::NotFound,
            other => DbError::Sqlx(other),
        }
    }

    /// Turn a Postgres unique-violation (SQLSTATE 23505) into `Conflict`,
    /// carrying the constraint name, so an insert of a duplicate email/slug maps
    /// to a field error rather than a 500.
    pub fn from_insert(e: sqlx::Error) -> Self {
        if let sqlx::Error::Database(db) = &e
            && db.code().as_deref() == Some("23505")
        {
            let constraint = db.constraint().unwrap_or("unique").to_string();
            return DbError::Conflict(constraint);
        }
        DbError::Sqlx(e)
    }
}
