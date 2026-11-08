//! Spot Finder SERP run (Lot B). For each KEPT keyword of a project, ask who
//! ranks (DataForSEO, paid), aggregate the ranking domains into spots, and store
//! them. Cost is controlled two ways: only kept keywords are queried (capped),
//! and the mutualised cross-tenant `provider_cache` skips any keyword+locale
//! looked up recently by ANY workspace (the margin lever, §6).

use backlio_core::provider_cache as cache_keys;
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
/// Max spots stored per project (already ranked strongest-first).
const MAX_SPOTS: usize = 100;

/// Run one SERP search (job `find_spots`). Claims the run, queries the kept
/// keywords (cache-first), aggregates, and replaces the project's spots.
pub async fn run_search(pool: &sqlx::PgPool, run_id: Uuid) -> anyhow::Result<()> {
    use db::repositories::{projects, provider_cache, spots};

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
        // Cache-first: a fresh cached SERP for this keyword+locale avoids a
        // bill — and the cache is shared across every workspace (§6).
        let cache_key = cache_keys::serp_key(keyword, &run.locale);
        let results: Vec<SerpResult> =
            match provider_cache::get(pool, cache_keys::SERP, &cache_key).await? {
                Some(v) => serde_json::from_value(v).unwrap_or_default(),
                None => match provider.organic(keyword, locale, SERP_DEPTH).await {
                    Ok(r) => {
                        serp_queries += 1;
                        let v = serde_json::to_value(&r)?;
                        provider_cache::put(
                            pool,
                            cache_keys::SERP,
                            &cache_key,
                            Some(run.locale.as_str()),
                            &v,
                            1,
                            cache_keys::SERP_TTL_SECS,
                        )
                        .await?;
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

    // Best (position, ranking page) per (spot host, keyword) from the raw hits —
    // the ground truth the thematic scores (PTA/PTP) are computed from.
    let mut per_kw: HashMap<(String, String), (i32, String)> = HashMap::new();
    for h in &hits {
        let host = site_host(&h.domain);
        per_kw
            .entry((host, h.keyword.clone()))
            .and_modify(|e| {
                if h.rank < e.0 {
                    *e = (h.rank, h.url.clone());
                }
            })
            .or_insert((h.rank, h.url.clone()));
    }

    let new_spots: Vec<spots::NewSpot> = top
        .iter()
        .map(|a| {
            let positions = per_kw
                .iter()
                .filter(|((host, _), _)| host == &a.domain)
                .map(|((_, kw), (rank, url))| spots::NewSpotPosition {
                    keyword: kw.clone(),
                    position: *rank,
                    page_url: Some(url.clone()),
                })
                .collect();
            spots::NewSpot {
                domain: &a.domain,
                keyword_count: a.keyword_count as i32,
                best_position: Some(a.best_position),
                avg_position: Some(a.avg_position),
                example_url: Some(&a.example_url),
                example_keyword: Some(&a.example_keyword),
                positions,
            }
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

    // "Your ranking is ready" — notify the org owner once the spot ranking is
    // built. Best-effort and env-gated (Resend): a mail failure never fails the
    // run. Only when the run actually produced a ranking.
    if !new_spots.is_empty()
        && let Ok(Some(to)) = db::repositories::orgs::owner_email(pool, org).await
    {
        let app_url = std::env::var("APP_URL")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "https://linkpulz.com".to_string());
        let subject = format!("Your link-spot ranking for {own_domain} is ready");
        let text = format!(
            "We finished analysing {own_domain}.\n\n\
             {count} link spots are ranked by how well they fit your topics, with their \
             marketplace prices to compare.\n\n\
             See your ranking: {app_url}/spots\n",
            count = new_spots.len(),
        );
        if let Err(e) = notifications::resend::send_email(&to, &subject, &text).await {
            tracing::warn!("ranking-ready email failed: {e}");
        }
    }
    Ok(())
}
