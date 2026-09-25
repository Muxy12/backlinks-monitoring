//! Expansion crawler (spec §21): given a KNOWN referring domain (a seed), crawl
//! a bounded set of its pages to find the ones that actually link to the
//! project's site, and record them as real Tier-1 backlinks. This does NOT
//! discover unknown referring domains (that needs a web-scale reverse index) —
//! it turns domain-level hints (e.g. a GSC "Top linking sites" export) into
//! source→target rows. SSRF-safe (reuses `fetch`), bounded by page count.

use backlio_core::{NormalizeOptions, host_of, normalize};
use crawler::{CrawlerConfig, FetchOutcome, fetch, parse_page};
use db::repositories::seeds;
use sqlx::PgPool;
use std::collections::{HashSet, VecDeque};

/// Re-crawl cadence for a finished seed, and the retry after an error.
const SEED_DONE_SECS: i64 = 7 * 24 * 3600;
const SEED_ERROR_SECS: i64 = 24 * 3600;

/// Process up to `max_seeds` due seeds, crawling at most `max_pages` each.
/// Returns the number of new backlinks discovered. Only call where crawling is
/// allowed (production) — it makes outbound HTTP requests.
pub async fn discover_tick(
    pool: &PgPool,
    cfg: &CrawlerConfig,
    max_seeds: i64,
    max_pages: usize,
) -> anyhow::Result<usize> {
    let due = seeds::claim_due(pool, max_seeds).await?;
    let mut total_new = 0usize;
    for seed in due {
        let proj: Option<(String,)> =
            sqlx::query_as("SELECT domain FROM projects WHERE id = $1 AND organization_id = $2")
                .bind(seed.project_id)
                .bind(seed.organization_id)
                .fetch_optional(pool)
                .await?;
        let Some((proj_domain,)) = proj else {
            seeds::finish(
                pool,
                seed.id,
                0,
                0,
                SEED_DONE_SECS,
                Some("project not found"),
            )
            .await?;
            continue;
        };
        let opts = NormalizeOptions::default();
        let target_reg = normalize(&format!("https://{proj_domain}"), None, opts)
            .and_then(|u| host_of(&u))
            .unwrap_or(proj_domain);

        match crawl_seed(pool, cfg, &seed, &target_reg, max_pages).await {
            Ok((pages, found)) => {
                total_new += found;
                seeds::finish(
                    pool,
                    seed.id,
                    pages as i32,
                    found as i32,
                    SEED_DONE_SECS,
                    None,
                )
                .await?;
            }
            Err(e) => {
                seeds::finish(pool, seed.id, 0, 0, SEED_ERROR_SECS, Some(&e.to_string())).await?;
            }
        }
    }
    Ok(total_new)
}

async fn crawl_seed(
    pool: &PgPool,
    cfg: &CrawlerConfig,
    seed: &db::entities::DiscoverySeed,
    target_reg: &str,
    max_pages: usize,
) -> anyhow::Result<(usize, usize)> {
    let opts = NormalizeOptions::default();
    let seed_reg = seed.normalized_domain.as_str();

    let mut queue: VecDeque<String> = VecDeque::new();
    queue.push_back(format!("https://{seed_reg}/"));
    let mut visited: HashSet<String> = HashSet::new();
    let mut pages = 0usize;
    let mut found = 0usize;

    while let Some(url) = queue.pop_front() {
        if pages >= max_pages {
            break;
        }
        if !visited.insert(url.clone()) {
            continue;
        }
        let ok = match fetch(cfg, &url).await {
            FetchOutcome::Response(r) if (200..300).contains(&r.status) => r,
            _ => continue,
        };
        if !ok
            .content_type
            .as_deref()
            .map(|c| c.contains("html"))
            .unwrap_or(true)
        {
            continue;
        }
        pages += 1;

        let page = parse_page(&ok.body);
        let base = page.base_href.as_deref().unwrap_or(&ok.final_url);
        for link in &page.links {
            let Some(href_norm) = normalize(&link.href_raw, Some(base), opts) else {
                continue;
            };
            let host_reg = host_of(&href_norm).unwrap_or_default();
            if host_reg == target_reg {
                // A link from this page to the project's site — a real backlink.
                let rel = if link.rel.is_dofollow(page.robots_nofollow) {
                    "dofollow"
                } else {
                    "nofollow"
                };
                if seeds::record_backlink(
                    pool,
                    seed.organization_id,
                    seed.project_id,
                    &ok.final_url,
                    &href_norm,
                    link.anchor_text.as_deref(),
                    rel,
                )
                .await
                .unwrap_or(false)
                {
                    found += 1;
                }
            } else if host_reg == seed_reg
                && !visited.contains(&href_norm)
                && visited.len() + queue.len() < max_pages
            {
                // Internal link — crawl a little deeper, bounded.
                queue.push_back(href_norm);
            }
        }
    }
    Ok((pages, found))
}
