//! Automatic monitoring: the scheduler enqueues due checks deduped by source
//! URL (§27), and the deduped handler checks every backlink sharing that page
//! from one fetch. Gated on `DATABASE_URL`.

use backlio_core::TransitionPolicy;
use crawler::CrawlerConfig;
use db::authorize_org;
use db::repositories::{backlinks, monitor, orgs, users};
use jobs::BackoffPolicy;
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use worker::{WorkerCtx, schedule_tick};

async fn ctx() -> Option<WorkerCtx> {
    let url = std::env::var("DATABASE_URL")
        .ok()
        .filter(|s| !s.is_empty())?;
    let db = db::build_pool(&db::DbConfig::worker(&url, 4, None))
        .await
        .ok()?;
    db::migrate::run(&db.pool).await.ok()?;
    Some(WorkerCtx {
        pool: db.pool,
        crawler: CrawlerConfig {
            allow_private_addresses: true,
            ..CrawlerConfig::default()
        },
        policy: TransitionPolicy::default(),
        backoff: BackoffPolicy::default(),
        worker_id: "test".into(),
    })
}

fn uniq(p: &str) -> String {
    format!("{p}-{}", Uuid::now_v7().simple())
}

#[tokio::test]
async fn scheduler_dedupes_by_source_and_handler_checks_all() {
    let Some(ctx) = ctx().await else { return };
    let pool = &ctx.pool;

    // One page, two target links on it.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/post"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            b"<a href=\"https://acme.com/a\">A</a><a href=\"https://acme.com/b\">B</a>".to_vec(),
            "text/html",
        ))
        .mount(&server)
        .await;
    let source_url = format!("{}/post", server.uri());

    let user = users::create(
        pool,
        Uuid::now_v7(),
        &format!("{}@x.com", uniq("u")),
        "h",
        "U",
    )
    .await
    .unwrap();
    let org = orgs::create_with_owner(pool, Uuid::now_v7(), "O", &uniq("o"), user.id)
        .await
        .unwrap();
    // A new workspace gets a 10-day trial in production (the create-org handler
    // calls this); the scheduler only monitors orgs with active access (docs/20).
    db::repositories::billing::start_trial(pool, org.id)
        .await
        .unwrap();
    let scope = authorize_org(pool, user.id, org.id).await.unwrap();
    let project = db::repositories::projects::create(
        pool,
        &scope,
        Uuid::now_v7(),
        "S",
        "acme.com",
        7 * 24 * 3600,
        None,
    )
    .await
    .unwrap();

    // Two backlinks sharing the same source page, different targets.
    let new = |target: &'static str| backlinks::NewBacklink {
        project_id: project.id,
        campaign_id: None,
        publisher_id: None,
        source_url: &source_url,
        target_url: target,
        expected_anchor_text: None,
        expected_rel_type: "dofollow",
        cost: None,
        currency: None,
        warranty_type: "none",
        warranty_value: None,
        warranty_expiration: None,
    };
    let a = backlinks::create(pool, &scope, Uuid::now_v7(), new("https://acme.com/a"))
        .await
        .unwrap();
    let b = backlinks::create(pool, &scope, Uuid::now_v7(), new("https://acme.com/b"))
        .await
        .unwrap();

    // Scheduler: both are due (next_check_at = now() on create). They share a
    // normalized source, so ONE job should carry both ids (§27).
    let (_jobs, scheduled) = schedule_tick(&ctx, 500, 3600).await.unwrap();
    assert!(scheduled >= 2, "both due backlinks were scheduled");

    // Exactly one check_source_page job exists for this source, with both ids.
    let (count,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM jobs WHERE job_type='check_source_page' AND payload->>'source_url' = $1",
    )
    .bind(&source_url)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(count, 1, "one deduped job for the shared source page");

    let (ids_len,): (i32,) = sqlx::query_as(
        "SELECT jsonb_array_length(payload->'backlink_ids') FROM jobs \
         WHERE job_type='check_source_page' AND payload->>'source_url' = $1",
    )
    .bind(&source_url)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(ids_len, 2i32, "the one job covers both backlinks");

    // The deduped handler checks both from a single fetch -> both live.
    let applied = worker::engine::check_source_group(
        pool,
        &ctx.crawler,
        &ctx.policy,
        &source_url,
        &[a.id, b.id],
    )
    .await
    .unwrap();
    assert_eq!(applied, 2);
    assert_eq!(
        monitor::load_state(pool, a.id).await.unwrap().status,
        "live"
    );
    assert_eq!(
        monitor::load_state(pool, b.id).await.unwrap().status,
        "live"
    );
}
