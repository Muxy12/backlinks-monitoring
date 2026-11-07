//! Link-buying marketplace lookup (Spot Finder, Lot D). A pluggable seam: given
//! a spot's domain, ask each configured marketplace whether it is listed and at
//! what price, so a spot can be marked "buyable". LinkAvista is the first
//! provider; more add here without touching the caller. Providers are built from
//! env secrets and are simply absent when unconfigured.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Filters for browsing a marketplace catalogue (the source-agnostic subset).
#[derive(Debug, Clone, Default)]
pub struct CatalogueQuery {
    /// Free-text / theme query (e.g. a project's topic or kept keyword).
    pub search: Option<String>,
    /// Market/language, e.g. `fr` | `en`.
    pub locale: Option<String>,
    /// Minimum domain authority/rank to include.
    pub min_domain_rank: Option<i32>,
    /// Maximum placement price (minor units).
    pub max_price_cents: Option<i64>,
    /// Max results to pull per source.
    pub limit: usize,
}

/// One buyable site from a marketplace catalogue — a spot candidate sourced from
/// the marketplace's own API (not SERP). Metrics are best-effort per source;
/// `None` when the source doesn't expose a field (never fabricated).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogueSpot {
    /// The marketplace this came from (e.g. "linkavista").
    pub source: String,
    pub domain: String,
    pub price_cents: Option<i64>,
    pub currency: Option<String>,
    pub offer_url: Option<String>,
    pub site_id: Option<i64>,
    /// Authority metric as the source reports it (scale varies by source).
    pub domain_rank: Option<i32>,
    pub referring_domains: Option<i32>,
    pub organic_traffic: Option<i64>,
    pub theme: Option<String>,
    pub dofollow: Option<bool>,
}

/// A marketplace listing for a domain (the fields a spot surfaces). The terms
/// beyond price (dofollow / content / indexation / retention) feed the
/// price-compare view; each is `None` when the source doesn't expose it — we
/// never fabricate a term.
#[derive(Debug, Clone)]
pub struct MarketplaceListing {
    pub site_id: Option<i64>,
    /// Placement price in the platform's money unit (EUR cents for LinkAvista).
    pub price_cents: Option<i64>,
    pub currency: Option<String>,
    pub url: Option<String>,
    /// Whether the placement is a dofollow link.
    pub dofollow: Option<bool>,
    /// Whether article writing is included in the price.
    pub content_included: Option<bool>,
    /// Whether the source guarantees the page is indexed.
    pub indexation_guarantee: Option<bool>,
    /// Minimum months the link is kept live.
    pub retention_months: Option<i32>,
}

#[async_trait]
pub trait MarketplaceProvider: Send + Sync {
    /// Stable identifier stored on the spot (e.g. "linkavista").
    fn name(&self) -> &'static str;
    /// Look up `domain` in this marketplace's catalogue; `None` if not listed.
    async fn lookup(&self, domain: &str) -> anyhow::Result<Option<MarketplaceListing>>;
    /// Browse the marketplace catalogue — the buyable sites matching `q`. This is
    /// how spots are now sourced (from the marketplace API, not SERP).
    async fn search(&self, q: &CatalogueQuery) -> anyhow::Result<Vec<CatalogueSpot>>;
}

/// Browse every configured marketplace's catalogue and aggregate the results
/// (best-effort: a source that errors is skipped, not fatal). Caching belongs to
/// the caller (worker/route) via `provider_cache`, which has the DB handle.
pub async fn search_all(q: &CatalogueQuery) -> Vec<CatalogueSpot> {
    let mut out = Vec::new();
    for p in providers_from_env() {
        if let Ok(mut spots) = p.search(q).await {
            out.append(&mut spots);
        }
    }
    out
}

/// Whether any marketplace source is configured (so the match flow can gate on
/// "any source" rather than a specific one).
pub fn any_configured() -> bool {
    !providers_from_env().is_empty()
}

/// Build the configured marketplace providers from env. Currently LinkAvista
/// (`LINKAVISTA_API_KEY` / optional `LINKAVISTA_BASE_URL`); future platforms are
/// appended here.
pub fn providers_from_env() -> Vec<Box<dyn MarketplaceProvider>> {
    let mut out: Vec<Box<dyn MarketplaceProvider>> = Vec::new();
    let key = std::env::var("LINKAVISTA_API_KEY")
        .ok()
        .filter(|s| !s.is_empty());
    let base = std::env::var("LINKAVISTA_BASE_URL")
        .ok()
        .filter(|s| !s.is_empty());
    if let Some(client) = crate::linkavista::from_cfg(key.as_deref(), base.as_deref()) {
        out.push(Box::new(LinkavistaMarketplace { client }));
    }
    out
}

struct LinkavistaMarketplace {
    client: crate::linkavista::LinkavistaClient,
}

#[async_trait]
impl MarketplaceProvider for LinkavistaMarketplace {
    fn name(&self) -> &'static str {
        "linkavista"
    }

    async fn lookup(&self, domain: &str) -> anyhow::Result<Option<MarketplaceListing>> {
        let host = backlio_core::site_host(domain);
        let page = self
            .client
            .list_sites(&crate::linkavista::SiteQuery {
                search: Some(host.clone()),
                per_page: Some(50),
                ..Default::default()
            })
            .await?;
        for s in page.data {
            if backlio_core::site_host(&s.domain) == host {
                let pricing = s.pricing;
                return Ok(Some(MarketplaceListing {
                    site_id: Some(s.id),
                    price_cents: pricing.as_ref().and_then(|p| p.price),
                    currency: pricing.and_then(|p| p.currency),
                    url: s.url,
                    // LinkAvista exposes these via writing_modes/constraints;
                    // not mapped yet, so left unknown rather than guessed.
                    dofollow: None,
                    content_included: None,
                    indexation_guarantee: None,
                    retention_months: None,
                }));
            }
        }
        Ok(None)
    }

    async fn search(&self, q: &CatalogueQuery) -> anyhow::Result<Vec<CatalogueSpot>> {
        let page = self
            .client
            .list_sites(&crate::linkavista::SiteQuery {
                search: q.search.clone(),
                language: q.locale.clone(),
                da_min: q.min_domain_rank.map(|v| v.max(0) as u32),
                price_max_cents: q.max_price_cents.map(|v| v.max(0) as u64),
                per_page: Some(q.limit.clamp(1, 100) as u32),
                ..Default::default()
            })
            .await?;
        Ok(page
            .data
            .into_iter()
            .map(|s| {
                let metrics = s.metrics;
                let pricing = s.pricing;
                CatalogueSpot {
                    source: "linkavista".to_string(),
                    domain: backlio_core::site_host(&s.domain),
                    price_cents: pricing.as_ref().and_then(|p| p.price),
                    currency: pricing.and_then(|p| p.currency),
                    offer_url: s.url,
                    site_id: Some(s.id),
                    domain_rank: metrics.as_ref().and_then(|m| m.da).map(|v| v as i32),
                    referring_domains: metrics.as_ref().and_then(|m| m.rd).map(|v| v as i32),
                    organic_traffic: metrics.as_ref().and_then(|m| m.total_traffic),
                    theme: s.theme,
                    dofollow: None,
                }
            })
            .collect())
    }
}
