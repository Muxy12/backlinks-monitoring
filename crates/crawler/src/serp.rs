//! DataForSEO SERP lookup (Spot Finder, Lot B). Given a keyword and a locale,
//! return the Google organic results (domain, rank, page URL). Paid and
//! env-gated: built from `DATAFORSEO_LOGIN` / `DATAFORSEO_PASSWORD` (the same
//! credentials the backlinks discovery provider uses) and simply absent
//! (feature-off) when unconfigured, so nothing is called and nothing is billed
//! until keys are set.

use serde::{Deserialize, Serialize};

/// One organic SERP result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SerpResult {
    pub url: String,
    pub domain: String,
    /// Organic rank (1 = top of the organic results).
    pub rank: i32,
    pub title: Option<String>,
}

/// A locale for a SERP query: a DataForSEO location code + language code.
#[derive(Debug, Clone, Copy)]
pub struct Locale {
    pub location_code: i64,
    pub language_code: &'static str,
}

impl Locale {
    /// Map our short locale tag (`fr` | `en`) to DataForSEO codes. Unknown tags
    /// fall back to France/French.
    pub fn from_tag(tag: &str) -> Locale {
        match tag {
            "en" => Locale {
                location_code: 2840, // United States
                language_code: "en",
            },
            _ => Locale {
                location_code: 2250, // France
                language_code: "fr",
            },
        }
    }
}

/// DataForSEO Google Organic SERP client.
pub struct SerpProvider {
    login: String,
    password: String,
}

impl SerpProvider {
    /// Build from env, or `None` when `DATAFORSEO_LOGIN`/`DATAFORSEO_PASSWORD`
    /// are not both set.
    pub fn from_env() -> Option<Self> {
        let nonempty = |k: &str| std::env::var(k).ok().filter(|s| !s.is_empty());
        Some(SerpProvider {
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

    /// Fetch the organic results for one keyword. `depth` is clamped to [10,100].
    /// POST /v3/serp/google/organic/live/advanced (live = one synchronous call).
    pub async fn organic(
        &self,
        keyword: &str,
        locale: Locale,
        depth: usize,
    ) -> anyhow::Result<Vec<SerpResult>> {
        let body = serde_json::json!([{
            "keyword": keyword,
            "location_code": locale.location_code,
            "language_code": locale.language_code,
            "depth": depth.clamp(10, 100),
        }]);
        let resp = Self::client()
            .post("https://api.dataforseo.com/v3/serp/google/organic/live/advanced")
            .basic_auth(&self.login, Some(&self.password))
            .json(&body)
            .send()
            .await?;
        if !resp.status().is_success() {
            // Surface DataForSEO's own message (bad key, no funds, no SERP-API
            // access) rather than a bare status.
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            let msg = serde_json::from_str::<serde_json::Value>(&text)
                .ok()
                .and_then(|v| v["status_message"].as_str().map(str::to_string))
                .unwrap_or_else(|| text.chars().take(200).collect());
            anyhow::bail!("dataforseo serp HTTP {status}: {msg}");
        }
        let v: serde_json::Value = resp.json().await?;
        // 200 even for task-level failures; the real outcome is in status_code
        // (20000 = ok). Bail with the message so an empty list isn't mistaken for
        // "0 results".
        if v["status_code"].as_i64() != Some(20000) {
            anyhow::bail!(
                "dataforseo serp: {} (code {})",
                v["status_message"].as_str().unwrap_or("error"),
                v["status_code"].as_i64().unwrap_or(0)
            );
        }
        let task = v["tasks"].get(0);
        if let Some(t) = task
            && t["status_code"].as_i64() != Some(20000)
        {
            anyhow::bail!(
                "dataforseo serp task: {} (code {})",
                t["status_message"].as_str().unwrap_or("error"),
                t["status_code"].as_i64().unwrap_or(0)
            );
        }
        let items = task
            .and_then(|t| t["result"].get(0))
            .and_then(|r| r["items"].as_array())
            .cloned()
            .unwrap_or_default();

        let mut out = Vec::new();
        for it in items {
            // Only true organic results carry a ranking position we can use;
            // skip features (maps, people-also-ask, ads, …).
            if it["type"].as_str() != Some("organic") {
                continue;
            }
            let url = it["url"].as_str().unwrap_or("").to_string();
            let domain = it["domain"].as_str().unwrap_or("").to_string();
            if url.is_empty() || domain.is_empty() {
                continue;
            }
            // rank_group = position among organic results; fall back to absolute.
            let rank = it["rank_group"]
                .as_i64()
                .or_else(|| it["rank_absolute"].as_i64())
                .unwrap_or(0) as i32;
            out.push(SerpResult {
                url,
                domain,
                rank,
                title: it["title"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .map(str::to_string),
            });
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_mapping() {
        assert_eq!(Locale::from_tag("en").location_code, 2840);
        assert_eq!(Locale::from_tag("fr").language_code, "fr");
        assert_eq!(Locale::from_tag("zz").location_code, 2250); // fallback
    }
}
