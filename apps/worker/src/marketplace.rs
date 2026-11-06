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

    let targets = spots::spots_to_enrich(pool, org, project, false, MAX_SPOTS).await?;
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
        // Ask EVERY configured source (no first-wins break) and collect each
        // one that lists this domain — that is what enables price-compare.
        let mut listings: Vec<(&'static str, crawler::marketplace::MarketplaceListing)> =
            Vec::new();
        for p in &providers {
            queries += 1;
            match p.lookup(&domain).await {
                Ok(Some(listing)) => listings.push((p.name(), listing)),
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

        // Persist the normalised offers (one row per source), replacing any
        // previous ones for this spot.
        let mut offers: Vec<spots::NewOffer> = Vec::with_capacity(listings.len());
        for (name, l) in &listings {
            offers.push(spots::NewOffer {
                source_slug: *name,
                editor_domain: domain.as_str(),
                site_id: l.site_id,
                price_cents: l.price_cents,
                currency: l.currency.as_deref(),
                dofollow: l.dofollow,
                content_included: l.content_included,
                indexation_guarantee: l.indexation_guarantee,
                retention_months: l.retention_months,
                offer_url: l.url.as_deref(),
            });
        }
        spots::replace_spot_offers(pool, org, project, spot_id, &offers).await?;

        // Cache the headline (cheapest, dofollow-preferred) offer onto the spot
        // row so existing consumers (buyable flag, campaign, CSV) keep working.
        let compare: Vec<backlio_core::marketplace::Offer> = listings
            .iter()
            .map(|(_, l)| backlio_core::marketplace::Offer {
                price_cents: l.price_cents,
                dofollow: l.dofollow,
            })
            .collect();
        match backlio_core::marketplace::best_offer(&compare) {
            Some(i) => {
                matches += 1;
                let (name, l) = &listings[i];
                spots::update_spot_marketplace(
                    pool,
                    spot_id,
                    &spots::SpotMarketplace {
                        marketplace: Some(*name),
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
