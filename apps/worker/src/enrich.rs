//! Spot Finder enrichment (Lot C). Stage 2 of the funnel: for the shortlist of
//! spots (top by relevance), fetch authority/trust/traffic/keyword-breadth
//! metrics (DataForSEO) and compute the explainable quality score. Only the
//! shortlist is enriched, so the cost stays bounded.

use backlio_core::{SpotSignals, spot_score};
use crawler::metrics::MetricsProvider;
use uuid::Uuid;

/// How many top spots to enrich per run (cost guard).
const TOP_SPOTS: i64 = 30;

/// Run one enrichment pass (job `enrich_spots`).
pub async fn run_enrich(pool: &sqlx::PgPool, run_id: Uuid) -> anyhow::Result<()> {
    use db::repositories::spots;

    let Some(run) = spots::mark_enrich_running(pool, run_id).await? else {
        return Ok(());
    };
    let org = run.organization_id;
    let project = run.project_id;

    let provider = match MetricsProvider::from_env() {
        Some(p) => p,
        None => {
            spots::finish_enrich(pool, run_id, 0, 0, Some("DataForSEO is not configured")).await?;
            return Ok(());
        }
    };

    // only_stale = true: skip spots already enriched within the last week so a
    // repeated run doesn't re-bill DataForSEO for fresh metrics.
    let targets = spots::spots_to_enrich(pool, org, project, true, TOP_SPOTS).await?;
    if targets.is_empty() {
        spots::finish_enrich(
            pool,
            run_id,
            0,
            0,
            Some("all spots already have fresh metrics — nothing to enrich"),
        )
        .await?;
        return Ok(());
    }

    let mut enriched = 0i32;
    let mut provider_queries = 0i32;
    for (spot_id, domain, keyword_count) in targets {
        let metrics = match provider.domain_metrics(&domain).await {
            Ok(m) => m,
            Err(e) => {
                let msg = format!("{e}");
                spots::finish_enrich(pool, run_id, enriched, provider_queries, Some(msg.as_str()))
                    .await?;
                return Ok(());
            }
        };
        // Two provider calls per domain (backlinks summary + labs overview).
        provider_queries += 2;

        let signals = SpotSignals {
            keyword_count: keyword_count.max(0) as u32,
            domain_rank: metrics.domain_rank,
            referring_domains: metrics.referring_domains,
            spam_score: metrics.spam_score,
            organic_keywords: metrics.organic_keywords,
            organic_traffic: metrics.organic_traffic,
        };
        let sc = spot_score(&signals);

        spots::update_spot_metrics(
            pool,
            spot_id,
            &spots::SpotMetrics {
                domain_rank: metrics.domain_rank,
                referring_domains: metrics.referring_domains,
                spam_score: metrics.spam_score,
                organic_keywords: metrics.organic_keywords,
                organic_traffic: metrics.organic_traffic,
                quality_score: sc.total,
            },
        )
        .await?;
        enriched += 1;
    }

    spots::finish_enrich(pool, run_id, enriched, provider_queries, None).await?;
    Ok(())
}
