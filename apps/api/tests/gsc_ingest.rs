//! GSC Search Analytics ingestion repo layer (Post-Link Impact PR 2): a mapped
//! connection is listed as due, and the gsc_daily upsert is idempotent. The live
//! Google HTTP path is verified in prod; the URL encoding is unit-tested in
//! `api::gsc_sync`. Gated on `DATABASE_URL`.

use db::sqlx;
use uuid::Uuid;

async fn pool() -> Option<sqlx::PgPool> {
    let url = std::env::var("DATABASE_URL")
        .ok()
        .filter(|s| !s.is_empty())?;
    let db = db::build_pool(&db::DbConfig::worker(&url, 4, None))
        .await
        .ok()?;
    db::migrate::run(&db.pool).await.ok()?;
    Some(db.pool)
}

#[tokio::test]
async fn due_listing_and_idempotent_upsert() {
    let Some(pool) = pool().await else {
        return;
    };
    let org = Uuid::now_v7();
    let project = Uuid::now_v7();
    let conn_id = Uuid::now_v7();
    let slug = format!("gsci-{}", Uuid::now_v7().simple());

    sqlx::query("INSERT INTO organizations (id,name,slug) VALUES ($1,'t',$2)")
        .bind(org)
        .bind(&slug)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO projects (id,organization_id,name,domain) VALUES ($1,$2,'p','example.com')",
    )
    .bind(project)
    .bind(org)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO gsc_connections (id,organization_id,project_id,site_url,refresh_token_enc) \
         VALUES ($1,$2,$3,'sc-domain:example.com',$4)",
    )
    .bind(conn_id)
    .bind(org)
    .bind(project)
    .bind(vec![1u8, 2, 3, 4])
    .execute(&pool)
    .await
    .unwrap();

    // A never-synced, property-mapped connection is due.
    let stale = chrono::Utc::now() + chrono::Duration::hours(1);
    let due = db::repositories::gsc::list_due(&pool, stale, 50)
        .await
        .unwrap();
    assert!(
        due.iter()
            .any(|c| c.id == conn_id && c.site_url == "sc-domain:example.com"),
        "connection should be listed due"
    );

    // Upsert two days of page metrics.
    let pages = vec![
        "https://example.com/a".to_string(),
        "https://example.com/a".to_string(),
    ];
    let days = vec![
        chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        chrono::NaiveDate::from_ymd_opt(2026, 1, 2).unwrap(),
    ];
    db::repositories::gsc_daily::upsert_many(
        &pool,
        org,
        project,
        &pages,
        &days,
        &[10, 20],
        &[1, 2],
        &[0.1, 0.1],
        &[5.0, 4.0],
    )
    .await
    .unwrap();

    let (n1,): (i64,) =
        sqlx::query_as("SELECT count(*)::bigint FROM gsc_daily WHERE project_id=$1")
            .bind(project)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(n1, 2, "two (page, day) rows");

    // Re-running refreshes the same rows (no duplicates) and updates the value.
    db::repositories::gsc_daily::upsert_many(
        &pool,
        org,
        project,
        &pages,
        &days,
        &[99, 20],
        &[1, 2],
        &[0.2, 0.1],
        &[3.0, 4.0],
    )
    .await
    .unwrap();
    let (n2,): (i64,) =
        sqlx::query_as("SELECT count(*)::bigint FROM gsc_daily WHERE project_id=$1")
            .bind(project)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(n2, 2, "upsert is idempotent (still 2 rows)");
    let (imp,): (i32,) = sqlx::query_as(
        "SELECT impressions FROM gsc_daily WHERE project_id=$1 AND day='2026-01-01'",
    )
    .bind(project)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(imp, 99, "upsert updated the metric");

    // Cleanup (best-effort).
    let _ = sqlx::query("DELETE FROM organizations WHERE id=$1")
        .bind(org)
        .execute(&pool)
        .await;
}
