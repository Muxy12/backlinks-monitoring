//! DataForSEO domain metrics (Spot Finder, Lot C). Given a domain, fetch the
//! authority / trust / traffic / keyword-breadth signals used by the spot
//! scorecard. Two calls per domain (Backlinks summary + Labs domain rank
//! overview), so this only runs on the shortlist (stage 2 of the funnel). Paid
//! and env-gated on the same `DATAFORSEO_LOGIN`/`DATAFORSEO_PASSWORD`.

/// Metrics for one domain. Each field is best-effort — `None` when the provider
/// didn't return it — so a partial result still enriches what it can.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DomainMetrics {
    /// DataForSEO rank, 0..=1000 (authority).
    pub domain_rank: Option<i32>,
    pub referring_domains: Option<i32>,
    /// Backlink spam score, 0..=100 (lower is better).
    pub spam_score: Option<i32>,
    /// Keywords the domain ranks for across Google.
    pub organic_keywords: Option<i32>,
    /// Estimated monthly organic traffic.
    pub organic_traffic: Option<i64>,
}

/// DataForSEO metrics client (shares credentials with the SERP/backlinks providers).
pub struct MetricsProvider {
    login: String,
    password: String,
}

impl MetricsProvider {
    pub fn from_env() -> Option<Self> {
        let nonempty = |k: &str| std::env::var(k).ok().filter(|s| !s.is_empty());
        Some(MetricsProvider {
            login: nonempty("DATAFORSEO_LOGIN")?,
            password: nonempty("DATAFORSEO_PASSWORD")?,
        })
    }

    fn client() -> reqwest::Client {
        reqwest::Client::builder()
            .user_agent("LinkPulz/1.0")
            .timeout(std::time::Duration::from_secs(45))
            .build()
            .unwrap_or_default()
    }

    async fn post(&self, url: &str, body: serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let resp = Self::client()
            .post(url)
            .basic_auth(&self.login, Some(&self.password))
            .json(&body)
            .send()
            .await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            let msg = serde_json::from_str::<serde_json::Value>(&text)
                .ok()
                .and_then(|v| v["status_message"].as_str().map(str::to_string))
                .unwrap_or_else(|| text.chars().take(200).collect());
            anyhow::bail!("dataforseo HTTP {status}: {msg}");
        }
        let v: serde_json::Value = resp.json().await?;
        if v["status_code"].as_i64() != Some(20000) {
            anyhow::bail!(
                "dataforseo: {} (code {})",
                v["status_message"].as_str().unwrap_or("error"),
                v["status_code"].as_i64().unwrap_or(0)
            );
        }
        Ok(v)
    }

    /// Fetch authority + traffic metrics for `domain`. Runs the two calls and
    /// merges them; a failure in either bubbles up so the caller can surface it.
    pub async fn domain_metrics(&self, domain: &str) -> anyhow::Result<DomainMetrics> {
        let target = bare_host(domain);
        let mut m = DomainMetrics::default();

        // 1) Backlinks summary → authority (rank), referring domains, spam score.
        let summary = self
            .post(
                "https://api.dataforseo.com/v3/backlinks/summary/live",
                serde_json::json!([{ "target": target, "include_subdomains": true }]),
            )
            .await?;
        if let Some(item) = first_result_item(&summary) {
            m.domain_rank = item["rank"].as_i64().map(|n| n as i32);
            m.referring_domains = item["referring_domains"].as_i64().map(|n| n as i32);
            m.spam_score = item["backlinks_spam_score"].as_i64().map(|n| n as i32);
        }

        // 2) Labs domain rank overview → organic keyword count + est. traffic.
        let labs = self
            .post(
                "https://api.dataforseo.com/v3/dataforseo_labs/google/domain_rank_overview/live",
                serde_json::json!([{
                    "target": target,
                    "location_code": 2250,   // France; traffic estimate is directional
                    "language_code": "fr",
                }]),
            )
            .await?;
        if let Some(item) = first_result_item(&labs) {
            let organic = &item["metrics"]["organic"];
            m.organic_keywords = organic["count"].as_i64().map(|n| n as i32);
            // etv = estimated traffic value (visits/month), a float.
            m.organic_traffic = organic["etv"].as_f64().map(|t| t.round() as i64);
        }

        Ok(m)
    }
}

/// First `tasks[0].result[0].items[0]` of a DataForSEO response, if present.
fn first_result_item(v: &serde_json::Value) -> Option<&serde_json::Value> {
    v["tasks"]
        .get(0)?
        .get("result")?
        .get(0)?
        .get("items")?
        .get(0)
}

/// Reduce a stored domain/URL to a bare host (same rule as the discovery provider).
fn bare_host(input: &str) -> String {
    let s = input.trim();
    let s = s.rsplit("://").next().unwrap_or(s);
    let s = s.split(['/', '?', '#']).next().unwrap_or(s);
    let s = s.rsplit('@').next().unwrap_or(s);
    let s = s.split(':').next().unwrap_or(s);
    let s = s.strip_prefix("www.").unwrap_or(s);
    s.to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_host_strips() {
        assert_eq!(bare_host("https://www.Example.com/x"), "example.com");
    }

    #[test]
    fn first_item_navigates_the_envelope() {
        let v = serde_json::json!({
            "tasks": [{ "result": [{ "items": [{ "rank": 500 }] }] }]
        });
        assert_eq!(first_result_item(&v).unwrap()["rank"], 500);
        assert!(first_result_item(&serde_json::json!({ "tasks": [] })).is_none());
    }
}
