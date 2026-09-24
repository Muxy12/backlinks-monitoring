//! Migration runner and schema-version guard (ADR-0005, §42).
//!
//! Migrations are **embedded** at compile time from `/migrations`. Application
//! binaries call [`verify`] at boot to refuse to start against an unmigrated or
//! newer-than-expected schema; they never call [`run`] themselves. `run` exists
//! for the gated CI/admin path and for tests, and must go through the DIRECT
//! (non-pooler) endpoint (docs/10 §4).

use crate::error::DbError;
use sqlx::PgPool;

/// The embedded migration set (compiled from the workspace `migrations/` dir).
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");

/// Apply all pending migrations. Gated CI / admin / test use only — never from
/// an application boot, never through the pooler (ADR-0005).
pub async fn run(pool: &PgPool) -> Result<(), DbError> {
    MIGRATOR.run(pool).await?;
    Ok(())
}

/// Verify the database is at the schema version this binary was built for, and
/// refuse to start otherwise. Returns the number of applied migrations.
///
/// "Applied but newer than we know" is an error too: a rolled-back deploy must
/// not run against a schema a later version created (ADR-0005 forward-only /
/// two-step rule).
pub async fn verify(pool: &PgPool) -> Result<usize, DbError> {
    let applied: Vec<(i64,)> =
        sqlx::query_as("SELECT version FROM _sqlx_migrations ORDER BY version")
            .fetch_all(pool)
            .await
            .map_err(DbError::from)?;

    let known: Vec<i64> = MIGRATOR.iter().map(|m| m.version).collect();

    for m in MIGRATOR.iter() {
        if !applied.iter().any(|(v,)| *v == m.version) {
            return Err(DbError::Config(format!(
                "database is missing migration {} ({}); run migrations before starting",
                m.version, m.description
            )));
        }
    }
    if let Some((newest,)) = applied.last() {
        if !known.contains(newest) {
            return Err(DbError::Config(format!(
                "database schema version {newest} is newer than this binary knows; \
                 deploy the matching code or roll the schema back"
            )));
        }
    }
    Ok(applied.len())
}
