//! LinkAvista marketplace API client (advertiser catalogue).
//!
//! LinkAvista is a backlink marketplace (buy/sell sponsored links). This reads
//! its advertiser catalogue so LinkPulz can surface buyable placements. The
//! client is built from a per-deployment API key and is simply absent
//! (feature-off) when the key is unconfigured, so nothing is called until it's
//! set. Lot 1 is READ-ONLY (catalogue browse); ordering is a later increment.
//!
//! Auth is a static key in the `X-API-Key` header. The key is a server-side
//! secret and must never be sent to the browser; the API layer proxies these
//! reads. Catalogue reads are free (only orders spend credit), so a browse
//! endpoint is acceptable; rate limit is ~200/min per key.

use serde::{Deserialize, Serialize};

const DEFAULT_BASE_URL: &str = "https://linkavista.com/api/v1";

pub struct LinkavistaClient {
    api_key: String,
    base_url: String,
}

/// Build a client, or `None` when no API key is configured (feature-off). The
/// base URL falls back to the public LinkAvista API when unset.
pub fn from_cfg(api_key: Option<&str>, base_url: Option<&str>) -> Option<LinkavistaClient> {
    let api_key = api_key
        .map(str::trim)
        .filter(|s| !s.is_empty())?
        .to_string();
    let base_url = base_url
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_BASE_URL)
        .trim_end_matches('/')
        .to_string();
    Some(LinkavistaClient { api_key, base_url })
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent("LinkPulz/1.0")
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .unwrap_or_default()
}

/// Filters for the advertiser catalogue (a safe subset of LinkAvista's params).
#[derive(Debug, Default, Clone)]
pub struct SiteQuery {
    pub da_min: Option<u32>,
    pub tf_min: Option<u32>,
    /// Max price in EUR cents, excl. VAT (LinkAvista money unit).
    pub price_max_cents: Option<u64>,
    pub language: Option<String>,
    pub search: Option<String>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SiteMetrics {
    pub da: Option<i64>,
    pub tf: Option<i64>,
    pub cf: Option<i64>,
    pub rd: Option<i64>,
    pub total_traffic: Option<i64>,
    pub keyword_count: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SitePricing {
    /// Base placement price in EUR cents, excl. VAT.
    pub price: Option<i64>,
    pub redaction_price: Option<i64>,
    pub currency: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct WritingModes {
    #[serde(default)]
    pub platform: bool,
    #[serde(default)]
    pub advertiser: bool,
    #[serde(default)]
    pub editor: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SiteConstraints {
    pub max_links: Option<i64>,
    pub min_words: Option<i64>,
}

/// A catalogue site, projected to the fields LinkPulz needs (not LinkAvista's
/// raw payload). Extra fields on the wire are ignored.
#[derive(Debug, Serialize, Deserialize)]
pub struct CatalogueSite {
    pub id: i64,
    pub domain: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub theme: Option<String>,
    #[serde(default)]
    pub theme_score: Option<i64>,
    #[serde(default)]
    pub metrics: Option<SiteMetrics>,
    #[serde(default)]
    pub pricing: Option<SitePricing>,
    #[serde(default)]
    pub writing_modes: Option<WritingModes>,
    #[serde(default)]
    pub constraints: Option<SiteConstraints>,
}

/// One page of catalogue results (the fields we surface from LinkAvista's
/// Laravel pagination envelope).
#[derive(Debug, Serialize)]
pub struct SitesPage {
    pub data: Vec<CatalogueSite>,
    pub current_page: i64,
    pub last_page: i64,
    pub total: i64,
    pub per_page: i64,
}

/// A placement order. Lot 3a supports platform-written articles only
/// (content_provider = "platform"), with one anchor → one target URL.
#[derive(Debug, Clone)]
pub struct OrderRequest {
    pub site_id: i64,
    pub content_provider: String,
    pub redaction_option: Option<String>,
    pub article_title: String,
    pub brief: String,
    pub anchor_text: String,
    pub target_url: String,
    pub publish_date: String,
}

/// The outcome of a create-order call. In sandbox this is a dry-run preview
/// (`sandbox = true`, no order created, no debit); in live mode it carries the
/// created order's id and status.
#[derive(Debug, Serialize)]
pub struct OrderOutcome {
    pub sandbox: bool,
    pub status: Option<String>,
    pub would_create_status: Option<String>,
    pub would_debit_cents: Option<i64>,
    pub price_cents: Option<i64>,
    pub currency: Option<String>,
    pub order_id: Option<String>,
    pub message: Option<String>,
}

impl LinkavistaClient {
    /// `GET /advertiser/sites` — the browsable catalogue (read-only, no spend).
    pub async fn list_sites(&self, q: &SiteQuery) -> anyhow::Result<SitesPage> {
        let mut url = reqwest::Url::parse(&format!("{}/advertiser/sites", self.base_url))?;
        {
            let mut qp = url.query_pairs_mut();
            let per_page = q.per_page.unwrap_or(24).clamp(1, 200);
            qp.append_pair("per_page", &per_page.to_string());
            if let Some(p) = q.page {
                qp.append_pair("page", &p.to_string());
            }
            if let Some(v) = q.da_min {
                qp.append_pair("da_min", &v.to_string());
            }
            if let Some(v) = q.tf_min {
                qp.append_pair("tf_min", &v.to_string());
            }
            if let Some(v) = q.price_max_cents {
                qp.append_pair("price_max", &v.to_string());
            }
            if let Some(v) = q
                .language
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                qp.append_pair("language[]", v);
            }
            if let Some(v) = q.search.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                qp.append_pair("search", v);
            }
        }

        let resp = client()
            .get(url)
            .header("X-API-Key", self.api_key.as_str())
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            // Surface LinkAvista's own error (bad key, IP not allowed, quota)
            // rather than a generic failure.
            let msg = serde_json::from_str::<serde_json::Value>(&text)
                .ok()
                .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(str::to_string))
                .unwrap_or_else(|| text.chars().take(200).collect());
            anyhow::bail!("linkavista HTTP {status}: {msg}");
        }

        let v: serde_json::Value = resp.json().await?;
        let data: Vec<CatalogueSite> =
            serde_json::from_value(v.get("data").cloned().unwrap_or(serde_json::Value::Null))
                .unwrap_or_default();
        let as_i = |k: &str, d: i64| v.get(k).and_then(|x| x.as_i64()).unwrap_or(d);
        Ok(SitesPage {
            data,
            current_page: as_i("current_page", 1),
            last_page: as_i("last_page", 1),
            total: as_i("total", 0),
            per_page: as_i("per_page", 24),
        })
    }

    /// `POST /advertiser/orders` — place an order. With a sandbox key this is a
    /// dry-run (no order, no debit); the caller must still gate live spending.
    pub async fn create_order(&self, o: &OrderRequest) -> anyhow::Result<OrderOutcome> {
        let body = serde_json::json!({
            "site_id": o.site_id,
            "content_provider": o.content_provider,
            "redaction_option": o.redaction_option,
            "article_title": o.article_title,
            "brief": o.brief,
            "anchors": [{ "anchor_text": o.anchor_text, "target_url": o.target_url }],
            "publish_date": o.publish_date,
        });

        let resp = client()
            .post(format!("{}/advertiser/orders", self.base_url))
            .header("X-API-Key", self.api_key.as_str())
            .header(reqwest::header::ACCEPT, "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            let msg = serde_json::from_str::<serde_json::Value>(&text)
                .ok()
                .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(str::to_string))
                .unwrap_or_else(|| text.chars().take(200).collect());
            anyhow::bail!("linkavista HTTP {status}: {msg}");
        }

        let v: serde_json::Value = resp.json().await?;
        let message = v
            .get("message")
            .and_then(|x| x.as_str())
            .map(str::to_string);
        let d = v.get("data").cloned().unwrap_or(serde_json::Value::Null);
        let get_s = |k: &str| d.get(k).and_then(|x| x.as_str()).map(str::to_string);
        let get_i = |k: &str| d.get(k).and_then(|x| x.as_i64());
        let order_id = d
            .get("order_id")
            .and_then(|x| x.as_i64())
            .map(|n| n.to_string())
            .or_else(|| get_s("order_id"))
            .or_else(|| d.get("id").and_then(|x| x.as_i64()).map(|n| n.to_string()));
        Ok(OrderOutcome {
            sandbox: d.get("sandbox").and_then(|x| x.as_bool()).unwrap_or(false),
            status: get_s("status"),
            would_create_status: get_s("would_create_status"),
            would_debit_cents: get_i("would_debit_cents"),
            price_cents: get_i("price").or_else(|| get_i("total_cents")),
            currency: get_s("currency"),
            order_id,
            message,
        })
    }
}
