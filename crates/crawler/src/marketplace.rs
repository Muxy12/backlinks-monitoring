//! Link-buying marketplace lookup (Spot Finder, Lot D). A pluggable seam: given
//! a spot's domain, ask each configured marketplace whether it is listed and at
//! what price, so a spot can be marked "buyable". LinkAvista is the first
//! provider; more add here without touching the caller. Providers are built from
//! env secrets and are simply absent when unconfigured.

use async_trait::async_trait;

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
}
