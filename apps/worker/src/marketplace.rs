//! Spot Finder marketplace matching (Lot D). Cross-reference each spot's domain
//! against the configured link-buying marketplaces (LinkAvista …) and mark the
//! buyable ones with their price. Catalogue reads don't spend credit, so this is
//! cheap; it still runs on demand (not every search) and is capped.

use crawler::marketplace::providers_from_env;
use uuid::Uuid;

/// Max spots checked per run (by relevance order).
const MAX_SPOTS: i64 = 100;

/// Run one marketplace-matching pass (job `match_marketplaces`).
pub async fn run_match(pool: &sqlx::PgPool, run_id: Uuid) -> anyhow::Result<()> {
    use db::repositories::spots;

    let Some(run) = spots::mark_match_running(pool, run_id).await? else {
        return Ok(());
    };
    let org = run.organization_id;
    let project = run.project_id;

    let providers = providers_from_env();
    if providers.is_empty() {
        spots::finish_match(
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

    let targets = spots::spots_to_enrich(pool, org, project, MAX_SPOTS).await?;
    if targets.is_empty() {
        spots::finish_match(
            pool,
            run_id,
            0,
            0,
            0,
            Some("no spots to match — search first"),
        )
        .await?;
        return Ok(());
    }

    let mut checked = 0i32;
    let mut matches = 0i32;
    let mut queries = 0i32;
    for (spot_id, domain, _kw) in targets {
        checked += 1;
        let mut listed: Option<(&'static str, crawler::marketplace::MarketplaceListing)> = None;
        for p in &providers {
            queries += 1;
            match p.lookup(&domain).await {
                Ok(Some(listing)) => {
                    listed = Some((p.name(), listing));
                    break; // first marketplace that lists it wins
                }
                Ok(None) => {}
                Err(e) => {
                    // Surface a provider error (bad key, rate limit) on the run.
                    let msg = format!("{e}");
                    spots::finish_match(
                        pool,
                        run_id,
                        checked,
                        matches,
                        queries,
                        Some(msg.as_str()),
                    )
                    .await?;
                    return Ok(());
                }
            }
        }
        match listed {
            Some((name, l)) => {
                matches += 1;
                spots::update_spot_marketplace(
                    pool,
                    spot_id,
                    &spots::SpotMarketplace {
                        marketplace: Some(name),
                        site_id: l.site_id,
                        price_cents: l.price_cents,
                        currency: l.currency.as_deref(),
                        url: l.url.as_deref(),
                    },
                )
                .await?;
            }
            None => {
                // Not listed anywhere now — clear any stale match.
                spots::update_spot_marketplace(pool, spot_id, &spots::SpotMarketplace::default())
                    .await?;
            }
        }
    }

    spots::finish_match(pool, run_id, checked, matches, queries, None).await?;
    Ok(())
}
