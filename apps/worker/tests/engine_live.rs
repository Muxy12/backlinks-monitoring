//! End-to-end monitoring: seed a backlink whose source page (a wiremock server)
//! contains the target link, run the check engine, and assert the state machine
//! and persistence made it `live` with a check row and a BACKLINK_FOUND event.
//! Gated on `DATABASE_URL`; the crawler runs with the test-only loopback escape
//! hatch so it can reach 127.0.0.1.

use backlio_core::TransitionPolicy;
use crawler::CrawlerConfig;
use db::authorize_org;
use db::repositories::{backlinks, monitor, orgs, users};
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

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
async fn check_makes_a_present_backlink_live_with_check_and_event() {
    let Some(pool) = pool().await else { return };

    // Source page linking to the target.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/article"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(
                "<html><body><p>see <a href=\"https://acme.com/product\">great tool</a></p></body></html>"
                    .as_bytes()
                    .to_vec(),
                "text/html",
            ),
        )
        .mount(&server)
        .await;

    // Seed user → org → project → backlink (source = the wiremock page).
    let user = users::create(
        &pool,
        Uuid::now_v7(),
        &format!("{}@x.com", uniq("u")),
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
    )
    .await
    .unwrap();

    let source_url = format!("{}/article", server.uri());
    let backlink = backlinks::create(
        &pool,
        &scope,
        Uuid::now_v7(),
        backlinks::NewBacklink {
            project_id: project.id,
            campaign_id: None,
            publisher_id: None,
            source_url: &source_url,
            target_url: "https://acme.com/product",
            expected_anchor_text: Some("great tool"),
            expected_rel_type: "dofollow",
            cost: None,
            currency: None,
        },
    )
    .await
    .unwrap();

    // Run the engine (loopback escape hatch so 127.0.0.1 is reachable in tests).
    let cfg = CrawlerConfig {
        allow_private_addresses: true,
        ..CrawlerConfig::default()
    };
    let applied = worker::engine::process_backlink_check(
        &pool,
        &cfg,
        &TransitionPolicy::default(),
        backlink.id,
    )
    .await
    .expect("engine ran");
    assert!(applied, "a check was applied");

    // The backlink is now live (present, dofollow, anchor matches).
    let st = monitor::load_state(&pool, backlink.id).await.unwrap();
    assert_eq!(st.status, "live", "present dofollow link -> live");
    assert_eq!(st.detected_is_dofollow, Some(true));
    assert_eq!(st.detected_anchor_text.as_deref(), Some("great tool"));

    // A check row and a BACKLINK_FOUND event were written.
    let (checks,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM backlink_checks WHERE backlink_id=$1")
            .bind(backlink.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(checks, 1);
    let (found_events,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM backlink_events WHERE backlink_id=$1 AND event_type='backlink_found'",
    )
    .bind(backlink.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(found_events, 1, "first detection emits BACKLINK_FOUND");
}

#[tokio::test]
async fn check_of_absent_link_does_not_lose_it_on_first_miss() {
    let Some(pool) = pool().await else { return };

    // Page that does NOT contain the target.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/empty"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            b"<html><body>nothing here</body></html>".to_vec(),
            "text/html",
        ))
        .mount(&server)
        .await;

    let user = users::create(
        &pool,
        Uuid::now_v7(),
        &format!("{}@x.com", uniq("u")),
        "hash",
        "U",
    )
    .await
    .unwrap();
    let org = orgs::create_with_owner(&pool, Uuid::now_v7(), "Org", &uniq("org"), user.id)
        .await
        .unwrap();
    let scope = authorize_org(&pool, user.id, org.id).await.unwrap();
    let project = db::repositories::projects::create(
        &pool,
        &scope,
        Uuid::now_v7(),
        "S",
        "acme.com",
        7 * 24 * 3600,
    )
    .await
    .unwrap();
    let backlink = backlinks::create(
        &pool,
        &scope,
        Uuid::now_v7(),
        backlinks::NewBacklink {
            project_id: project.id,
            campaign_id: None,
            publisher_id: None,
            source_url: &format!("{}/empty", server.uri()),
            target_url: "https://acme.com/product",
            expected_anchor_text: None,
            expected_rel_type: "dofollow",
            cost: None,
            currency: None,
        },
    )
    .await
    .unwrap();

    let cfg = CrawlerConfig {
        allow_private_addresses: true,
        ..CrawlerConfig::default()
    };
    worker::engine::process_backlink_check(&pool, &cfg, &TransitionPolicy::default(), backlink.id)
        .await
        .unwrap();

    // §24: one absence does NOT mark it lost. From `pending`, an unconfirmed
    // absence holds the public status at pending, not lost.
    let st = monitor::load_state(&pool, backlink.id).await.unwrap();
    assert_ne!(
        st.status, "lost",
        "a single absence must never produce lost (§24)"
    );
    assert_eq!(
        st.consecutive_failure_count, 1,
        "the failure is counted, unconfirmed"
    );
}
