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
}
