//! GEO capture (Phase 2). For one tracked query, fetch the AI answer's cited
//! sources (DataForSEO AI Overviews), record them, and recompute which cited
//! domains are placeable. Cache-first via the mutualised `provider_cache`, so a
//! query answered once serves every workspace for 0 provider units (§6). Paid
//! and env-gated; a no-op when DataForSEO is not configured.

use backlio_core::provider_cache as cache_keys;
use crawler::geo::{Citation, GeoProvider};
use crawler::serp::Locale;
use uuid::Uuid;

/// Run one GEO capture (job `capture_geo`) for a single tracked query.
pub async fn run_capture(pool: &sqlx::PgPool, geo_query_id: Uuid) -> anyhow::Result<()> {
    use db::repositories::{geo, provider_cache};

    // The query may have been deleted/disabled between enqueue and run.
    let Some(q) = geo::query_by_id(pool, geo_query_id).await? else {
        return Ok(());
    };
    if !q.enabled {
        return Ok(());
    }

    let provider = match GeoProvider::from_env() {
        Some(p) => p,
        None => return Ok(()), // feature-off: nothing to call, nothing billed
    };

    // Cache-first (cross-tenant): a fresh AI answer for this query+locale avoids
    // a bill. `cache_ref` records that a capture was read from cache.
    let locale = Locale::from_tag(&q.locale);
    let cache_key = cache_keys::ai_overview_key(&q.query, &q.locale);
    let (citations, cache_ref): (Vec<Citation>, Option<String>) =
        match provider_cache::get(pool, cache_keys::AI_OVERVIEW, &cache_key).await? {
            Some(v) => (
                serde_json::from_value(v).unwrap_or_default(),
                Some(cache_key.clone()),
            ),
            None => {
                let fetched = provider.ai_overview(&q.query, locale).await?;
                let v = serde_json::to_value(&fetched)?;
                provider_cache::put(
                    pool,
                    cache_keys::AI_OVERVIEW,
                    &cache_key,
                    Some(q.locale.as_str()),
                    &v,
                    1,
                    cache_keys::AI_OVERVIEW_TTL_SECS,
                )
                .await?;
                (fetched, None)
            }
        };

    let news: Vec<geo::NewCitation> = citations
        .iter()
        .map(|c| geo::NewCitation {
            cited_url: &c.url,
            cited_domain: &c.domain,
            rank: Some(c.rank),
        })
        .collect();
    geo::record_capture(
        pool,
        q.organization_id,
        q.project_id,
        geo_query_id,
        &q.engine,
        cache_ref.as_deref(),
        &news,
    )
    .await?;

    // Join the freshly-cited domains to buyable availability.
    geo::recompute_placeable(pool, q.organization_id, q.project_id).await?;
    Ok(())
}
