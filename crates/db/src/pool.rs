//! Pool construction. Applies the ADR-0005 connection strategy: conservative
//! sizing, fail-fast acquire, connection recycling, and the R1 statement-cache
//! decision.

use crate::config::DbConfig;
use crate::error::DbError;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{ConnectOptions, PgPool};
use std::str::FromStr;

/// A connection pool plus the config it was built from. `Clone` is cheap
/// (`PgPool` is an `Arc` inside), so this is passed by value into app state.
#[derive(Clone)]
pub struct Db {
    pub pool: PgPool,
}

/// Build a pool from config. Does not connect eagerly beyond `min_connections`;
/// call [`crate::health::ping`] at startup to confirm reachability.
pub async fn build_pool(cfg: &DbConfig) -> Result<Db, DbError> {
    let connect_opts = PgConnectOptions::from_str(&cfg.url)
        .map_err(|e| DbError::Config(format!("bad DATABASE_URL: {e}")))?
        // R1: bound (or disable) the per-connection prepared-statement cache so
        // named statements cannot collide against a transaction-mode pooler
        // (docs/01 §2.3). `0` disables it entirely.
        .statement_cache_capacity(cfg.statement_cache_capacity)
        // Do not log every statement at INFO; queries can carry user data (S10).
        .log_statements(log::LevelFilter::Debug);

    let pool = PgPoolOptions::new()
        .max_connections(cfg.max_connections)
        .min_connections(cfg.min_connections)
        .acquire_timeout(cfg.acquire_timeout)
        .idle_timeout(cfg.idle_timeout)
        .max_lifetime(cfg.max_lifetime)
        .connect_with(connect_opts)
        .await?;

    Ok(Db { pool })
}
