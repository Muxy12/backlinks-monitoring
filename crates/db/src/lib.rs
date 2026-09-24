//! Database layer: connection strategy, migrations, health (ADR-0005).
//!
//! Repositories (tenant-scoped queries) build on top of this and land with
//! their entities. This crate owns the pool, the Neon-specific connection
//! decisions, and the migration runner.

pub mod config;
pub mod error;
pub mod health;
pub mod migrate;
pub mod pool;

pub use config::DbConfig;
pub use error::DbError;
pub use pool::{build_pool, Db};

/// Re-export so downstream crates use one sqlx version.
pub use sqlx;
