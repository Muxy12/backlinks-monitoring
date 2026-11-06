//! DataForSEO AI Overviews / AI-mode (GEO pillar, Phase 2). Given a query and a
//! locale, return the sources the AI answer cites (domain, rank, url) — the
//! wedge of the new positioning. Paid and env-gated on the same `DATAFORSEO_*`
//! credentials as SERP/metrics, and simply absent (feature-off) when unset, so
//! nothing is called and nothing is billed until keys are configured.
//!
//! The citation parsing is a pure function (`parse_citations`) so it is unit
//! tested offline against a response fixture; the provider is just the HTTP
//! around it. Never fabricates a citation — a source is emitted only when the
//! response actually carries its url + domain (docs/product/21 §9).

use crate::serp::Locale;
use serde::{Deserialize, Serialize};

/// One source cited by an AI answer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Citation {
    pub url: String,
    pub domain: String,
    /// Order the source appears among the answer's citations (1 = first).
    pub rank: i32,
    pub title: Option<String>,
}

/// DataForSEO AI-mode client (shares credentials with SERP/metrics).
pub struct GeoProvider {
    login: String,
    password: String,
}

impl GeoProvider {
    /// Build from env, or `None` when `DATAFORSEO_LOGIN`/`DATAFORSEO_PASSWORD`
    /// are not both set.
    pub fn from_env() -> Option<Self> {
        let nonempty = |k: &str| std::env::var(k).ok().filter(|s| !s.is_empty());
        Some(GeoProvider {
            login: nonempty("DATAFORSEO_LOGIN")?,
            password: nonempty("DATAFORSEO_PASSWORD")?,
        })
    }

    fn client() -> reqwest::Client {
        reqwest::Client::builder()
            .user_agent("LinkPulz/1.0")
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .unwrap_or_default()
    }

    /// Fetch the AI answer's cited sources for one query.
    /// POST /v3/serp/google/ai_mode/live/advanced (live = one synchronous call).
    pub async fn ai_overview(&self, query: &str, locale: Locale) -> anyhow::Result<Vec<Citation>> {
        let body = serde_json::json!([{
            "keyword": query,
            "location_code": locale.location_code,
            "language_code": locale.language_code,
        }]);
        let resp = Self::client()
            .post("https://api.dataforseo.com/v3/serp/google/ai_mode/live/advanced")
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
            anyhow::bail!("dataforseo ai_mode HTTP {status}: {msg}");
        }
        let v: serde_json::Value = resp.json().await?;
        if v["status_code"].as_i64() != Some(20000) {
            anyhow::bail!(
                "dataforseo ai_mode: {} (code {})",
                v["status_message"].as_str().unwrap_or("error"),
                v["status_code"].as_i64().unwrap_or(0)
            );
        }
        if let Some(t) = v["tasks"].get(0)
            && t["status_code"].as_i64() != Some(20000)
        {
            anyhow::bail!(
                "dataforseo ai_mode task: {} (code {})",
                t["status_message"].as_str().unwrap_or("error"),
                t["status_code"].as_i64().unwrap_or(0)
            );
        }
        Ok(parse_citations(&v))
    }
}

/// Pure: extract the cited sources from a DataForSEO AI-mode response envelope.
///
/// Walks `tasks[0].result[0].items[]` and, from each item, reads the AI answer's
/// cited links out of a `references` array (the AI-overview citation list) or,
/// failing that, a nested `items` array. A link counts only when it carries both
/// a non-empty `url` and `domain`. Deduped by url, ranked by first appearance.
pub fn parse_citations(v: &serde_json::Value) -> Vec<Citation> {
    let items = v["tasks"]
        .get(0)
        .and_then(|t| t["result"].get(0))
        .and_then(|r| r["items"].as_array())
        .cloned()
        .unwrap_or_default();

    let mut out: Vec<Citation> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for item in &items {
        // Prefer an explicit `references` list; fall back to nested `items`.
        let refs = item["references"]
            .as_array()
            .or_else(|| item["items"].as_array());
        let Some(refs) = refs else { continue };
        for r in refs {
            let url = r["url"].as_str().unwrap_or("");
            let domain = r["domain"].as_str().unwrap_or("");
            if url.is_empty() || domain.is_empty() || !seen.insert(url.to_string()) {
                continue;
            }
            out.push(Citation {
                url: url.to_string(),
                domain: domain.to_string(),
                rank: out.len() as i32 + 1,
                title: r["title"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .map(str::to_string),
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_references_in_order_and_dedups() {
        let v = serde_json::json!({
            "status_code": 20000,
            "tasks": [{
                "status_code": 20000,
                "result": [{
                    "items": [{
                        "type": "ai_overview",
                        "references": [
                            {"url": "https://a.com/x", "domain": "a.com", "title": "A"},
                            {"url": "https://b.com/y", "domain": "b.com"},
                            {"url": "https://a.com/x", "domain": "a.com"} // dup url
                        ]
                    }]
                }]
            }]
        });
        let cites = parse_citations(&v);
        assert_eq!(cites.len(), 2);
        assert_eq!(cites[0].domain, "a.com");
        assert_eq!(cites[0].rank, 1);
        assert_eq!(cites[0].title.as_deref(), Some("A"));
        assert_eq!(cites[1].domain, "b.com");
        assert_eq!(cites[1].rank, 2);
        assert_eq!(cites[1].title, None);
    }

    #[test]
    fn falls_back_to_nested_items() {
        let v = serde_json::json!({
            "tasks": [{ "result": [{ "items": [{
                "type": "ai_overview",
                "items": [{"url": "https://c.com", "domain": "c.com"}]
            }]}]}]
        });
        let cites = parse_citations(&v);
        assert_eq!(cites.len(), 1);
        assert_eq!(cites[0].domain, "c.com");
    }

    #[test]
    fn skips_links_missing_url_or_domain() {
        let v = serde_json::json!({
            "tasks": [{ "result": [{ "items": [{
                "references": [
                    {"url": "https://ok.com", "domain": "ok.com"},
                    {"url": "", "domain": "bad.com"},
                    {"domain": "nourl.com"},
                    {"url": "https://nodomain.com"}
                ]
            }]}]}]
        });
        let cites = parse_citations(&v);
        assert_eq!(cites.len(), 1);
        assert_eq!(cites[0].domain, "ok.com");
    }

    #[test]
    fn empty_envelope_yields_nothing() {
        assert!(parse_citations(&serde_json::json!({ "tasks": [] })).is_empty());
        assert!(parse_citations(&serde_json::json!({})).is_empty());
    }
}
