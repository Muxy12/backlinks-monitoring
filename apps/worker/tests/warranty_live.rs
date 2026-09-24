//! Warranty expiry sweep (§32): seed a backlink whose warranty lapses inside
//! the lookahead window, run the sweep, and assert exactly one alert is written
//! — and that a second run writes none (idempotent). Gated on `DATABASE_URL`.

use backlio_core::{WarrantyType, warranty_expiration};
use db::authorize_org;
use db::repositories::{backlinks, notifications, orgs, users};
use uuid::Uuid;
use worker::engine;

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

fn uniq(p: &str) -> String {
    format!("{p}-{}", Uuid::now_v7().simple())
}

#[tokio::test]
async fn warranty_sweep_alerts_once_and_is_idempotent() {
    let Some(pool) = pool().await else { return };

    let user = users::create(
        &pool,
        Uuid::now_v7(),
        &format!("{}@x.com", uniq("w")),
        "hash",
        "U",
    )
    .await
    .unwrap();
    let org = orgs::create_with_owner(&pool, Uuid::now_v7(), "Acme", &uniq("acme"), user.id)
        .await
        .unwrap();
    let scope = authorize_org(&pool, user.id, org.id).await.unwrap();
    let project = db::repositories::projects::create(
        &pool,
        &scope,
        Uuid::now_v7(),
        "Site",
        "acme.com",
        7 * 24 * 3600,
        None,
    )
    .await
    .unwrap();

    // A 30-day warranty acquired 25 days ago -> lapses in 5 days, inside a
    // 14-day lookahead. Expiration is materialized exactly as the API does it.
    let today = chrono::Utc::now().date_naive();
    let acquired = today - chrono::Duration::days(25);
    let exp = warranty_expiration(WarrantyType::Days, Some(30), acquired);
    assert!(exp.is_some());

    backlinks::create(
        &pool,
        &scope,
        Uuid::now_v7(),
        backlinks::NewBacklink {
            project_id: project.id,
            campaign_id: None,
            publisher_id: None,
            source_url: "https://publisher.com/post",
            target_url: "https://acme.com/product",
            expected_anchor_text: None,
            expected_rel_type: "dofollow",
            cost: None,
            currency: None,
            warranty_type: "days",
            warranty_value: Some(30),
            warranty_expiration: exp,
        },
    )
    .await
    .unwrap();

    // A far-future warranty must NOT alert yet.
    let far = warranty_expiration(WarrantyType::Days, Some(365), today);
    backlinks::create(
        &pool,
        &scope,
        Uuid::now_v7(),
        backlinks::NewBacklink {
            project_id: project.id,
            campaign_id: None,
            publisher_id: None,
            source_url: "https://publisher.com/other",
            target_url: "https://acme.com/product",
            expected_anchor_text: None,
            expected_rel_type: "dofollow",
            cost: None,
            currency: None,
            warranty_type: "days",
            warranty_value: Some(365),
            warranty_expiration: far,
        },
    )
    .await
    .unwrap();

    // The sweep is system-wide and the CI DB is shared across test binaries, so
    // the *global* return count isn't deterministic. Assert on this org's own
    // unread delta instead: exactly one alert for the soon-to-lapse link, and
    // the far-future link produces none.
    let before = notifications::unread_count(&pool, &scope).await.unwrap();

    let n1 = engine::run_warranty_sweep(&pool, today, 14, 5000)
        .await
        .unwrap();
    assert!(n1 >= 1, "sweep wrote at least our alert");
    let mid = notifications::unread_count(&pool, &scope).await.unwrap();
    assert_eq!(mid - before, 1, "exactly one alert for this org");

    // Second sweep same day: idempotent for this org (dedupe on backlink+date).
    engine::run_warranty_sweep(&pool, today, 14, 5000)
        .await
        .unwrap();
    let after = notifications::unread_count(&pool, &scope).await.unwrap();
    assert_eq!(after, mid, "re-run adds nothing for this org");
}
