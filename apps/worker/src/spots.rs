//! Spot Finder SERP run (Lot B). For each KEPT keyword of a project, ask who
//! ranks (DataForSEO, paid), aggregate the ranking domains into spots, and store
//! them. Cost is controlled two ways: only kept keywords are queried (capped),
//! and an org-scoped cache skips keywords looked up recently.

use backlio_core::{SerpHit, aggregate_spots, forum_kind, site_host};
use crawler::serp::{Locale, SerpProvider, SerpResult};
use std::collections::HashMap;
use uuid::Uuid;

/// Max forums/communities stored per run (Lot E).
const MAX_FORUMS: usize = 50;
/// Max kept keywords queried per run (cost guard).
const MAX_KEYWORDS: i64 = 50;
/// Organic depth requested per keyword.
const SERP_DEPTH: usize = 20;
/// Reuse a cached SERP for a keyword+locale for this many days.
const CACHE_DAYS: i64 = 14;
/// Max spots stored per project (already ranked strongest-first).
const MAX_SPOTS: usize = 100;

/// Run one SERP search (job `find_spots`). Claims the run, queries the kept
/// keywords (cache-first), aggregates, and replaces the project's spots.
pub async fn run_search(pool: &sqlx::PgPool, run_id: Uuid) -> anyhow::Result<()> {
    use db::repositories::{projects, spots};

    let Some(run) = spots::mark_run_running(pool, run_id).await? else {
        return Ok(());
    };
    let org = run.organization_id;
    let project = run.project_id;

    let provider = match SerpProvider::from_env() {
        Some(p) => p,
        None => {
            spots::finish_run(pool, run_id, 0, 0, 0, Some("DataForSEO is not configured")).await?;
            return Ok(());
        }
    };

    let own_domain = projects::domain_by_id(pool, org, project)
        .await?
        .unwrap_or_default();
    let keywords = spots::kept_keywords(pool, org, project, MAX_KEYWORDS).await?;
    if keywords.is_empty() {
        spots::finish_run(pool, run_id, 0, 0, 0, Some("no kept keywords to search")).await?;
        return Ok(());
    }

    let locale = Locale::from_tag(&run.locale);
    let mut hits: Vec<SerpHit> = Vec::new();
    let mut serp_queries = 0i32;
    let mut keywords_queried = 0i32;

    for keyword in &keywords {
        let key = keyword.to_lowercase();
        // Cache-first: a fresh cached SERP for this keyword+locale avoids a bill.
        let results: Vec<SerpResult> =
            match spots::cache_get(pool, org, &key, &run.locale, CACHE_DAYS).await? {
                Some(v) => serde_json::from_value(v).unwrap_or_default(),
                None => match provider.organic(keyword, locale, SERP_DEPTH).await {
                    Ok(r) => {
                        serp_queries += 1;
                        let v = serde_json::to_value(&r)?;
                        spots::cache_put(pool, org, &key, &run.locale, &v).await?;
                        r
                    }
                    Err(e) => {
                        // A provider error (bad key, no funds, no SERP access) is
                        // surfaced on the run rather than silently dropping results.
                        let msg = format!("{e}");
                        spots::finish_run(
                            pool,
                            run_id,
                            keywords_queried,
                            serp_queries,
                            0,
                            Some(msg.as_str()),
                        )
                        .await?;
                        return Ok(());
                    }
                },
            };
        keywords_queried += 1;
        for r in results {
            hits.push(SerpHit {
                keyword: keyword.clone(),
                url: r.url,
                domain: r.domain,
                rank: r.rank,
            });
        }
    }

    let aggregated = aggregate_spots(&hits, &own_domain);
    let top = &aggregated[..aggregated.len().min(MAX_SPOTS)];
    let new_spots: Vec<spots::NewSpot> = top
        .iter()
        .map(|a| spots::NewSpot {
            domain: &a.domain,
            keyword_count: a.keyword_count as i32,
            best_position: Some(a.best_position),
            avg_position: Some(a.avg_position),
            example_url: Some(&a.example_url),
            example_keyword: Some(&a.example_keyword),
        })
        .collect();
    spots::replace_spots(pool, org, project, &new_spots).await?;

    // Forums & communities (Lot E): split out the forum-like SERP hits — free,
    // same data. Dedup by URL, keep the best position + a keyword, drop own site.
    let own_host = site_host(&own_domain);
    struct ForumAcc {
        domain: String,
        kind: &'static str,
        best: i32,
        keyword: String,
    }
    let mut forums: HashMap<String, ForumAcc> = HashMap::new();
    for h in &hits {
        let Some(kind) = forum_kind(&h.url) else {
            continue;
        };
        let host = site_host(&h.domain);
        if host.is_empty() || host == own_host {
            continue;
        }
        let entry = forums.entry(h.url.clone()).or_insert_with(|| ForumAcc {
            domain: host,
            kind: kind.as_str(),
            best: h.rank,
            keyword: h.keyword.clone(),
        });
        if h.rank < entry.best {
            entry.best = h.rank;
            entry.keyword = h.keyword.clone();
        }
    }
    let mut forum_rows: Vec<(String, ForumAcc)> = forums.into_iter().collect();
    forum_rows.sort_by_key(|(_, f)| f.best);
    forum_rows.truncate(MAX_FORUMS);
    let new_forums: Vec<spots::NewForum> = forum_rows
        .iter()
        .map(|(url, f)| spots::NewForum {
            url,
            domain: &f.domain,
            forum_kind: f.kind,
            example_keyword: Some(&f.keyword),
            best_position: Some(f.best),
        })
        .collect();
    spots::replace_forums(pool, org, project, &new_forums).await?;

    spots::finish_run(
        pool,
        run_id,
        keywords_queried,
        serp_queries,
        new_spots.len() as i32,
        None,
    )
    .await?;
    Ok(())
}
