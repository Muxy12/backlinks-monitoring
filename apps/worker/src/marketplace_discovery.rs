//! Marketplace-first spot discovery. For a project, browse the configured
//! marketplaces' catalogues (their APIs) for the project's kept keywords, rank
//! the buyable sites by relevance + quality, and persist them as spots with
//! their offers. Catalogue reads don't spend provider credit, so this is cheap
//! and is NOT quota-gated. Reuses the spot-search run lifecycle.

use backlio_core::{SpotSignals, spot_score};
use crawler::marketplace::{CatalogueQuery, CatalogueSpot, any_configured, search_all};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// Kept keywords browsed per run (each is one catalogue query per source).
const MAX_KEYWORDS: i64 = 5;
/// Catalogue results pulled per keyword, per source.
const PER_KEYWORD_LIMIT: usize = 50;
/// Max spots persisted per project (ranked strongest-first).
const MAX_SPOTS: usize = 100;

/// Run one marketplace discovery pass (job `discover_marketplace_spots`).
pub async fn run_discovery(pool: &sqlx::PgPool, run_id: Uuid) -> anyhow::Result<()> {
    use db::repositories::spots;

    let Some(run) = spots::mark_run_running(pool, run_id).await? else {
        return Ok(());
    };
    let org = run.organization_id;
    let project = run.project_id;

    if !any_configured() {
        spots::finish_run(
            pool,
            run_id,
            0,
            0,
            0,
            Some("no link marketplace is configured"),
        )
        .await?;
        return Ok(());
    }

    let keywords = spots::kept_keywords(pool, org, project, MAX_KEYWORDS).await?;
    if keywords.is_empty() {
        spots::finish_run(
            pool,
            run_id,
            0,
            0,
            0,
            Some("keep at least one keyword first"),
        )
        .await?;
        return Ok(());
    }

    // Browse every source's catalogue for each kept keyword. Dedup by domain;
    // `kw_count` = how many distinct keywords surfaced a domain (its relevance),
    // `example_kw` keeps the first keyword that found it.
    let mut by_domain: HashMap<String, CatalogueSpot> = HashMap::new();
    let mut kw_count: HashMap<String, i32> = HashMap::new();
    let mut example_kw: HashMap<String, String> = HashMap::new();
    let mut queried = 0i32;

    for kw in &keywords {
        let q = CatalogueQuery {
            search: Some(kw.clone()),
            locale: Some(run.locale.clone()),
            limit: PER_KEYWORD_LIMIT,
            ..Default::default()
        };
        let found = search_all(&q).await;
        queried += 1;
        let mut seen_this_kw: HashSet<String> = HashSet::new();
        for c in found {
            let domain = c.domain.clone();
            if domain.is_empty() {
                continue;
            }
            if seen_this_kw.insert(domain.clone()) {
                *kw_count.entry(domain.clone()).or_insert(0) += 1;
                example_kw
                    .entry(domain.clone())
                    .or_insert_with(|| kw.clone());
            }
            by_domain.entry(domain).or_insert(c);
        }
    }

    // Score by the catalogue metrics + relevance (keyword_count), rank, cap.
    let mut ranked: Vec<(CatalogueSpot, i32, f64, String)> = by_domain
        .into_values()
        .map(|c| {
            let count = kw_count.get(&c.domain).copied().unwrap_or(0);
            let signals = SpotSignals {
                keyword_count: count.max(0) as u32,
                domain_rank: c.domain_rank,
                referring_domains: c.referring_domains,
                spam_score: None,
                organic_keywords: None,
                organic_traffic: c.organic_traffic,
            };
            let score = spot_score(&signals).total;
            let kw = example_kw.get(&c.domain).cloned().unwrap_or_default();
            (c, count, score, kw)
        })
        .collect();
    ranked.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    ranked.truncate(MAX_SPOTS);

    let new_spots: Vec<spots::NewCatalogueSpot> = ranked
        .iter()
        .map(|(c, count, score, kw)| spots::NewCatalogueSpot {
            domain: c.domain.as_str(),
            keyword_count: *count,
            example_keyword: Some(kw.as_str()),
            domain_rank: c.domain_rank,
            referring_domains: c.referring_domains,
            organic_traffic: c.organic_traffic,
            quality_score: *score,
            source_slug: c.source.as_str(),
            price_cents: c.price_cents,
            currency: c.currency.as_deref(),
            offer_url: c.offer_url.as_deref(),
            site_id: c.site_id,
        })
        .collect();

    let found = new_spots.len() as i32;
    spots::replace_catalogue_spots(pool, org, project, &new_spots).await?;
    spots::finish_run(pool, run_id, queried, 0, found, None).await?;
    Ok(())
}
