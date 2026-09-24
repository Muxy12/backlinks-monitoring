//! Liveness / readiness against the database. Used by `/ready` (docs/09 §2) and
//! at worker/API boot to confirm the pool can actually reach Neon.

use crate::error::DbError;
use sqlx::PgPool;

/// A trivial round-trip. Uses a runtime query (not the `query!` macro) so it
/// needs no compile-time database or `.sqlx` cache.
pub async fn ping(pool: &PgPool) -> Result<(), DbError> {
    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(pool)
        .await?;
    Ok(())
}
