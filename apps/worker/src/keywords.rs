//! Spot Finder keyword detection (Lot A). Crawl a project's OWN site — the
//! homepage (P0) and the pages one click away (P1) — extract the `<h1>` headings
//! as keyword candidates, and store them for the user to review before any paid
//! SERP lookup. No external provider and no cost: it is a crawl of the user's
//! own site through the same SSRF-safe fetch as monitoring.

use crawler::{CrawlerConfig, scan_site};
use std::collections::HashSet;
use std::time::Duration;
use uuid::Uuid;

/// Pages one click from the homepage to scan at most.
const MAX_P1_PAGES: usize = 20;
/// Cap on candidates stored per run, so a huge site can't flood the review list.
const MAX_CANDIDATES: usize = 80;

/// Run one keyword-detection scan (job `detect_site_keywords`). Claims the scan,
/// crawls the site, upserts the deduped candidates, and records the outcome. A
/// site we cannot reach is recorded as an error (not a retry-forever failure).
pub async fn run_detection(
    pool: &sqlx::PgPool,
    crawler: &CrawlerConfig,
    scan_id: Uuid,
) -> anyhow::Result<()> {
    use db::repositories::{projects, spot_keywords};

    // Claim the scan (queued -> running). Absent/already-claimed => nothing to do.
    let Some(scan) = spot_keywords::mark_running(pool, scan_id).await? else {
        return Ok(());
    };

    let home = match projects::domain_by_id(pool, scan.organization_id, scan.project_id).await? {
        Some(d) => home_url(&d),
        None => {
            spot_keywords::finish_scan(pool, scan_id, 0, 0, 0, Some("project not found")).await?;
            return Ok(());
        }
    };

    // Tighter timeouts than monitoring: a detection run fetches many pages, so a
    // single slow page must not stall the whole scan. The SSRF guard
    // (`allow_private_addresses`) is preserved by the struct update.
    let scan_cfg = CrawlerConfig {
        request_timeout: Duration::from_secs(10),
        total_timeout: Duration::from_secs(12),
        check_target: false,
        ..crawler.clone()
    };

    let result = scan_site(&scan_cfg, &home, MAX_P1_PAGES).await;
    let h1_count = result.h1s.len() as i32;

    // Clean + dedup into candidates, keeping the first origin page for each.
    let mut seen: HashSet<String> = HashSet::new();
    let mut inserted = 0i32;
    for h in &result.h1s {
        if inserted as usize >= MAX_CANDIDATES {
            break;
        }
        let Some(c) = backlio_core::clean_keyword(&h.text) else {
            continue;
        };
        if !seen.insert(c.normalized.clone()) {
            continue;
        }
        if spot_keywords::upsert_detected(
            pool,
            scan.organization_id,
            scan.project_id,
            &c.display,
            &c.normalized,
            Some(&h.url),
        )
        .await?
        {
            inserted += 1;
        }
    }

    // A run that reached no page at all is reported as an error, so the UI can
    // say "we couldn't reach your site" instead of "0 keywords".
    let err = (result.pages_scanned == 0).then_some("could not fetch the site homepage");
    spot_keywords::finish_scan(
        pool,
        scan_id,
        result.pages_scanned as i32,
        h1_count,
        inserted,
        err,
    )
    .await?;
    Ok(())
}

/// Project domains are stored with a scheme (`https://site.com`); tolerate a
/// bare host just in case.
fn home_url(domain: &str) -> String {
    let d = domain.trim();
    if d.contains("://") {
        d.to_string()
    } else {
        format!("https://{d}")
    }
}
