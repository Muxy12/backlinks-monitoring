//! Database layer: connection strategy, migrations, health (ADR-0005).
//!
//! Repositories (tenant-scoped queries) build on top of this and land with
//! their entities. This crate owns the pool, the Neon-specific connection
//! decisions, and the migration runner.

pub mod config;
pub mod entities;
pub mod error;
pub mod health;
pub mod migrate;
pub mod pool;
pub mod repositories;
pub mod scope;

pub use config::DbConfig;
pub use entities::{Backlink, BacklinkEvent, Client, Organization, Project, Session, User};
pub use error::DbError;
pub use pool::{Db, build_pool};
pub use scope::{OrgScope, authorize_org};

/// Re-export so downstream crates use one sqlx version.
pub use sqlx;
