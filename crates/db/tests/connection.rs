//! Integration tests that need a real Postgres. Gated on `DATABASE_URL`: when
//! it is unset (e.g. a local box with no database) the tests return early and
//! pass, so `cargo test` is never blocked. CI's `migrations` job sets
//! `DATABASE_URL` and exercises them for real (docs/13 Phase 1 exit criteria).
//!
//! This is also where open item **R1** gets settled: `pooler_statement_cache`
//! drives concurrent transactions with the cache ENABLED against whatever DSN
//! is provided. Point `NEON_TEST_POOLED_URL` at a real Neon `-pooler` endpoint
//! to learn whether SQLx's named statements are safe there (docs/adr/0005).

use db::config::{resolve_statement_cache_capacity, DbConfig};
use db::{build_pool, migrate};

fn dsn() -> Option<String> {
    std::env::var("DATABASE_URL").ok().filter(|s| !s.is_empty())
}

#[tokio::test]
async fn migrations_apply_and_ping_succeeds() {
    let Some(url) = dsn() else {
        eprintln!("DATABASE_URL unset — skipping (see crates/db/tests/connection.rs)");
        return;
    };
    // Direct-style config (cache on) — this DSN is the CI service container.
    let cfg = DbConfig::worker(&url, 4, None);
    let db = build_pool(&cfg).await.expect("pool builds");

    migrate::run(&db.pool).await.expect("migrations apply");
    db::health::ping(&db.pool).await.expect("ping works");

    // verify() must now find the schema at the version this binary knows.
    let applied = migrate::verify(&db.pool).await.expect("schema verifies");
    assert!(applied >= 12, "expected all migrations applied, got {applied}");
}

#[tokio::test]
async fn tenant_tables_exist_after_migration() {
    let Some(url) = dsn() else {
        return;
    };
    let db = build_pool(&DbConfig::worker(&url, 2, None)).await.unwrap();
    migrate::run(&db.pool).await.unwrap();

    // A couple of the spec §41 tables, by a runtime query (no compile-time DB).
    for table in ["organizations", "backlinks", "jobs", "backlink_events"] {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables \
             WHERE table_schema='public' AND table_name=$1)",
        )
        .bind(table)
        .fetch_one(&db.pool)
        .await
        .unwrap();
        assert!(exists, "table {table} should exist after migration");
    }
}

/// R1: drive concurrent transactions with the statement cache ENABLED. If SQLx's
/// named prepared statements collide against the pooler, this surfaces
/// `prepared statement "sqlx_s_N" already exists`. Only runs when
/// `NEON_TEST_POOLED_URL` is set (a real `-pooler` DSN); otherwise it is skipped,
/// because a local/direct Postgres cannot reproduce the pooler's behaviour.
#[tokio::test]
async fn pooler_statement_cache_survives_concurrency() {
    let Some(url) = std::env::var("NEON_TEST_POOLED_URL").ok().filter(|s| !s.is_empty()) else {
        eprintln!("NEON_TEST_POOLED_URL unset — skipping R1 pooler test (docs/adr/0005)");
        return;
    };
    // Force the cache ON regardless of host, to actually probe the risk.
    assert_eq!(resolve_statement_cache_capacity(&url, Some(50)), 50);
    let mut cfg = DbConfig::api(&url, 8, Some(50));
    cfg.statement_cache_capacity = 50;
    let db = build_pool(&cfg).await.expect("pooled pool builds");

    let mut handles = Vec::new();
    for _ in 0..32 {
        let pool = db.pool.clone();
        handles.push(tokio::spawn(async move {
            for _ in 0..10 {
                let mut tx = pool.begin().await?;
                let _n: i64 = sqlx::query_scalar("SELECT count(*) FROM information_schema.tables")
                    .fetch_one(&mut *tx)
                    .await?;
                tx.commit().await?;
            }
            Ok::<_, sqlx::Error>(())
        }));
    }
    for h in handles {
        h.await.expect("task joins").expect("no sqlx_s_ collision under the pooler");
    }
}
