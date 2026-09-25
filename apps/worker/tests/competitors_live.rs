//! Competitor ingestion (backlink-intelligence §1–§3). Gated on DATABASE_URL.

use db::authorize_org;
use db::repositories::{backlinks, competitors, orgs, projects, users};
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

fn uniq(p: &str) -> String {
    format!("{p}-{}", Uuid::now_v7().simple())
}

async fn count_events(pool: &sqlx::PgPool, org: Uuid, etype: &str) -> i64 {
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM backlink_events WHERE organization_id=$1 AND event_type::text=$2",
    )
    .bind(org)
    .bind(etype)
    .fetch_one(pool)
    .await
    .unwrap();
    n
}

#[tokio::test]
async fn competitor_ingestion_records_dedups_and_emits_events() {
    let Some(pool) = pool().await else { return };

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
    let project = projects::create(
        &pool,
        &scope,
        Uuid::now_v7(),
        "Site",
        "mysite.com",
        7 * 24 * 3600,
        None,
    )
    .await
    .unwrap();
    let comp = competitors::create(&pool, &scope, Uuid::now_v7(), project.id, "competitor.com")
        .await
        .unwrap();

    // First ingest -> new.
    let (_id, new1) = competitors::record_backlink(
        &pool,
        &scope,
        &comp,
        "https://news.com/a",
        "https://competitor.com/p",
        Some("great"),
        "dofollow",
    )
    .await
    .unwrap();
    assert!(new1, "first ingest is new");

    // Same relation again -> not new (dedup).
    let (_id2, new2) = competitors::record_backlink(
        &pool,
        &scope,
        &comp,
        "https://news.com/a",
        "https://competitor.com/p",
        Some("great"),
        "dofollow",
    )
    .await
    .unwrap();
    assert!(!new2, "re-ingest is not new");

    assert_eq!(
        count_events(&pool, org.id, "competitor_new_backlink").await,
        1
    );
    assert_eq!(
        count_events(&pool, org.id, "competitor_new_referring_domain").await,
        1
    );
    assert_eq!(
        count_events(&pool, org.id, "competitor_new_referring_page").await,
        1
    );

    // A second page from the SAME domain: new page, but not a new domain.
    competitors::record_backlink(
        &pool,
        &scope,
        &comp,
        "https://news.com/b",
        "https://competitor.com/q",
        None,
        "nofollow",
    )
    .await
    .unwrap();
    assert_eq!(
        count_events(&pool, org.id, "competitor_new_referring_domain").await,
        1,
        "same domain"
    );
    assert_eq!(
        count_events(&pool, org.id, "competitor_new_referring_page").await,
        2,
        "new page"
    );

    // Competitor rows never leak into the customer's own list; the competitor
    // list returns them.
    let own = backlinks::search(&pool, &scope, &backlinks::BacklinkFilter::default(), 200)
        .await
        .unwrap();
    assert!(
        own.iter().all(|b| b.subject == "own"),
        "own view excludes competitors"
    );
    let cbl = backlinks::search_competitor(&pool, &scope, comp.id, 500)
        .await
        .unwrap();
    assert_eq!(cbl.len(), 2, "both competitor backlinks listed");
    assert!(
        cbl.iter()
            .all(|b| b.subject == "competitor" && b.competitor_id == Some(comp.id))
    );
}

#[tokio::test]
async fn gap_analysis_finds_opportunity_domains() {
    let Some(pool) = pool().await else { return };
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
    let project = projects::create(
        &pool,
        &scope,
        Uuid::now_v7(),
        "Site",
        "mysite.com",
        7 * 24 * 3600,
        None,
    )
    .await
    .unwrap();
    let comp = competitors::create(&pool, &scope, Uuid::now_v7(), project.id, "competitor.com")
        .await
        .unwrap();

    // Our own backlink from sitez.com.
    backlinks::create(
        &pool,
        &scope,
        Uuid::now_v7(),
        backlinks::NewBacklink {
            project_id: project.id,
            campaign_id: None,
            publisher_id: None,
            source_url: "https://sitez.com/a",
            target_url: "https://mysite.com/p",
            expected_anchor_text: None,
            expected_rel_type: "dofollow",
            cost: None,
            currency: None,
            warranty_type: "none",
            warranty_value: None,
            warranty_expiration: None,
        },
    )
    .await
    .unwrap();

    // Competitor backlinks: sitex (only competitor) and sitez (also links us).
    competitors::record_backlink(
        &pool,
        &scope,
        &comp,
        "https://sitex.com/a",
        "https://competitor.com/p",
        None,
        "dofollow",
    )
    .await
    .unwrap();
    competitors::record_backlink(
        &pool,
        &scope,
        &comp,
        "https://sitez.com/b",
        "https://competitor.com/q",
        None,
        "dofollow",
    )
    .await
    .unwrap();

    let gap = competitors::gap_analysis(&pool, &scope, project.id, 1, 100)
        .await
        .unwrap();
    let domains: Vec<String> = gap.iter().map(|(d, _)| d.clone()).collect();
    assert!(
        domains.contains(&"sitex.com".to_string()),
        "sitex only links a competitor -> gap"
    );
    assert!(
        !domains.contains(&"sitez.com".to_string()),
        "sitez also links us -> not a gap"
    );
}

#[tokio::test]
async fn tier2_ingestion_links_to_the_tier1_source() {
    let Some(pool) = pool().await else { return };
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
    let project = projects::create(
        &pool,
        &scope,
        Uuid::now_v7(),
        "Site",
        "mysite.com",
        7 * 24 * 3600,
        None,
    )
    .await
    .unwrap();
    // Tier-1: blog.com/post -> mysite.com
    let t1 = backlinks::create(
        &pool,
        &scope,
        Uuid::now_v7(),
        backlinks::NewBacklink {
            project_id: project.id,
            campaign_id: None,
            publisher_id: None,
            source_url: "https://blog.com/post",
            target_url: "https://mysite.com/p",
            expected_anchor_text: None,
            expected_rel_type: "dofollow",
            cost: None,
            currency: None,
            warranty_type: "none",
            warranty_value: None,
            warranty_expiration: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(t1.tier, 1);

    // Tier-2: forum.com/thread -> blog.com/post (reinforces t1).
    let (_id, new1) = db::repositories::tiers::record_tier2(
        &pool,
        &scope,
        &t1,
        "https://forum.com/thread",
        Some("nice"),
        "dofollow",
    )
    .await
    .unwrap();
    assert!(new1);
    // Dedup.
    let (_id2, new2) = db::repositories::tiers::record_tier2(
        &pool,
        &scope,
        &t1,
        "https://forum.com/thread",
        Some("nice"),
        "dofollow",
    )
    .await
    .unwrap();
    assert!(!new2);

    let t2 = backlinks::search_tier2(&pool, &scope, t1.id, 500)
        .await
        .unwrap();
    assert_eq!(t2.len(), 1);
    assert_eq!(t2[0].tier, 2);
    assert_eq!(t2[0].parent_backlink_id, Some(t1.id));
    assert_eq!(
        t2[0].target_url, t1.source_url,
        "tier-2 targets the tier-1 source"
    );

    // The Tier-2 link is not in the Tier-1 (main) list.
    let own = backlinks::search(&pool, &scope, &backlinks::BacklinkFilter::default(), 200)
        .await
        .unwrap();
    assert!(own.iter().all(|b| b.tier == 1), "main list is Tier 1 only");
}

#[tokio::test]
async fn discovery_seeds_and_record_backlink() {
    let Some(pool) = pool().await else { return };
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
    let project = projects::create(
        &pool,
        &scope,
        Uuid::now_v7(),
        "Site",
        "mysite.com",
        7 * 24 * 3600,
        None,
    )
    .await
    .unwrap();

    // Seed create is idempotent per (project, domain).
    let s1 = db::repositories::seeds::create(
        &pool,
        &scope,
        Uuid::now_v7(),
        project.id,
        "ref.com",
        "gsc",
    )
    .await
    .unwrap();
    assert!(s1.is_some(), "first seed created");
    let s2 = db::repositories::seeds::create(
        &pool,
        &scope,
        Uuid::now_v7(),
        project.id,
        "ref.com",
        "gsc",
    )
    .await
    .unwrap();
    assert!(s2.is_none(), "duplicate seed skipped");

    // record_backlink inserts an own Tier-1 backlink, deduped.
    let new1 = db::repositories::seeds::record_backlink(
        &pool,
        org.id,
        project.id,
        "https://ref.com/a",
        "https://mysite.com/p",
        Some("hi"),
        "dofollow",
    )
    .await
    .unwrap();
    assert!(new1);
    let new2 = db::repositories::seeds::record_backlink(
        &pool,
        org.id,
        project.id,
        "https://ref.com/a",
        "https://mysite.com/p",
        Some("hi"),
        "dofollow",
    )
    .await
    .unwrap();
    assert!(!new2, "same discovered relation is not re-created");

    let own = backlinks::search(&pool, &scope, &backlinks::BacklinkFilter::default(), 200)
        .await
        .unwrap();
    assert!(
        own.iter()
            .any(|b| b.source_domain == "ref.com" && b.tier == 1 && b.subject == "own"),
        "discovered backlink shows in the own Tier-1 list"
    );

    // Claim + finish lifecycle.
    let sid = s1.unwrap().id;
    let due = db::repositories::seeds::claim_due(&pool, 50).await.unwrap();
    assert!(due.iter().any(|d| d.id == sid), "our seed is claimable");
    db::repositories::seeds::finish(&pool, sid, 3, 1, 3600, None)
        .await
        .unwrap();
}
