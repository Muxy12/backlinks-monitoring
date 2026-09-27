//! Pluggable backlink discovery providers (spec §21–§22).
//!
//! Recrawling known URLs cannot discover unknown backlinks — that needs an
//! external reverse index. This module is the seam for those sources. Each
//! provider turns a target domain into discovered `source → target` links; the
//! caller ingests them through the normal dedup/ingestion path. Providers are
//! built from env secrets and are simply absent (feature-off) when unconfigured,
//! so nothing is called and nothing is billed until keys are set.

use async_trait::async_trait;

/// One discovered backlink (pre-normalization; the DB layer normalizes + dedups).
#[derive(Debug, Clone)]
pub struct DiscoveredLink {
    pub source_url: String,
    pub target_url: String,
    pub anchor: Option<String>,
    pub dofollow: bool,
    /// Provider-supplied metadata (RFC-ish timestamps as the provider returns
    /// them; None when the provider doesn't supply them, e.g. the crawler).
    pub first_seen: Option<String>,
    pub last_seen: Option<String>,
    pub source_status: Option<i32>,
}

#[async_trait]
pub trait DiscoveryProvider: Send + Sync {
    fn name(&self) -> &'static str;
    /// Discover backlinks pointing at `target_domain` (bare domain), capped.
    async fn discover(
        &self,
        target_domain: &str,
        limit: usize,
    ) -> anyhow::Result<Vec<DiscoveredLink>>;
}

/// Build a provider from environment secrets, or `None` when not configured.
/// - `dataforseo` needs `DATAFORSEO_LOGIN` + `DATAFORSEO_PASSWORD`.
/// - `bing` needs `BING_WEBMASTER_API_KEY`.
pub fn from_env(name: &str) -> Option<Box<dyn DiscoveryProvider>> {
    let nonempty = |k: &str| std::env::var(k).ok().filter(|s| !s.is_empty());
    match name {
        "dataforseo" => {
            let login = nonempty("DATAFORSEO_LOGIN")?;
            let password = nonempty("DATAFORSEO_PASSWORD")?;
            Some(Box::new(DataForSeoProvider { login, password }))
        }
        "bing" => {
            let api_key = nonempty("BING_WEBMASTER_API_KEY")?;
            Some(Box::new(BingProvider { api_key }))
        }
        _ => None,
    }
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent("Backlio/1.0")
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_default()
}

/// DataForSEO Backlinks API (paid, documented) — competitors + unknown domains.
/// POST /v3/backlinks/backlinks/live with HTTP basic auth.
pub struct DataForSeoProvider {
    login: String,
    password: String,
}

#[async_trait]
impl DiscoveryProvider for DataForSeoProvider {
    fn name(&self) -> &'static str {
        "dataforseo"
    }

    async fn discover(
        &self,
        target_domain: &str,
        limit: usize,
    ) -> anyhow::Result<Vec<DiscoveredLink>> {
        // DataForSEO wants a bare domain for domain mode — no scheme/www/path.
        // Project domains are stored with the scheme (e.g. "https://site.com"),
        // so normalize before sending or the target is read as a page URL.
        let target = bare_host(target_domain);
        let body = serde_json::json!([{
            "target": target,
            "limit": limit.min(1000),
            "mode": "as_is",
            "backlinks_status_type": "live",
            "include_subdomains": true,
        }]);
        let resp = client()
            .post("https://api.dataforseo.com/v3/backlinks/backlinks/live")
            .basic_auth(&self.login, Some(&self.password))
            .json(&body)
            .send()
            .await?;
        if !resp.status().is_success() {
            anyhow::bail!("dataforseo HTTP {}", resp.status());
        }
        let v: serde_json::Value = resp.json().await?;
        let items = v["tasks"]
            .get(0)
            .and_then(|t| t["result"].get(0))
            .and_then(|r| r["items"].as_array())
            .cloned()
            .unwrap_or_default();
        let mut out = Vec::new();
        for it in items {
            // Only real backlink rows — skip redirects/canonicals/etc.
            if let Some(t) = it["item_type"].as_str()
                && t != "backlink"
            {
                continue;
            }
            let source_url = it["url_from"].as_str().unwrap_or("").to_string();
            let target_url = it["url_to"].as_str().unwrap_or("").to_string();
            if source_url.is_empty() || target_url.is_empty() {
                continue;
            }
            out.push(DiscoveredLink {
                source_url,
                target_url,
                anchor: it["anchor"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .map(str::to_string),
                dofollow: it["dofollow"].as_bool().unwrap_or(true),
                first_seen: it["first_seen"].as_str().map(str::to_string),
                last_seen: it["last_seen"].as_str().map(str::to_string),
                source_status: it["page_from_status_code"].as_i64().map(|n| n as i32),
            });
        }
        Ok(out)
    }
}

/// Bing Webmaster Tools API (free) — inbound links for YOUR verified site.
/// Best-effort mapping of `GetUrlLinks`; verify field names against your account
/// when activating (Bing's JSON shape varies). Gated by the API key.
pub struct BingProvider {
    api_key: String,
}

#[async_trait]
impl DiscoveryProvider for BingProvider {
    fn name(&self) -> &'static str {
        "bing"
    }

    async fn discover(
        &self,
        target_domain: &str,
        limit: usize,
    ) -> anyhow::Result<Vec<DiscoveredLink>> {
        let site = format!("https://{}/", bare_host(target_domain));
        let url = format!(
            "https://ssl.bing.com/webmaster/api.svc/json/GetUrlLinks?apikey={}&siteUrl={}&link=&page=0",
            urlencode(&self.api_key),
            urlencode(&site),
        );
        let resp = client().get(&url).send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("bing HTTP {}", resp.status());
        }
        let v: serde_json::Value = resp.json().await?;
        // Bing wraps results under "d"; the links array/field name has varied
        // across versions, so probe a few and read Url/AnchorText defensively.
        let arr = v["d"]
            .as_array()
            .or_else(|| v["d"]["Links"].as_array())
            .or_else(|| v["Links"].as_array())
            .cloned();
        let Some(arr) = arr else {
            anyhow::bail!("bing: unrecognized response shape — verify GetUrlLinks mapping");
        };
        let mut out = Vec::new();
        for it in arr.into_iter().take(limit) {
            let source = it["Url"]
                .as_str()
                .or_else(|| it["url"].as_str())
                .unwrap_or("");
            if source.is_empty() {
                continue;
            }
            out.push(DiscoveredLink {
                source_url: source.to_string(),
                target_url: site.clone(),
                anchor: it["AnchorText"]
                    .as_str()
                    .or_else(|| it["anchorText"].as_str())
                    .filter(|s| !s.is_empty())
                    .map(str::to_string),
                dofollow: true,
                first_seen: None,
                last_seen: None,
                source_status: None,
            });
        }
        Ok(out)
    }
}

/// Reduce a stored domain or URL to a bare registrable host: strip scheme,
/// userinfo, port, path/query/fragment and a leading `www.`, lowercased. Project
/// domains are stored with a scheme (`https://site.com`), but discovery providers
/// expect a bare host (`site.com`).
fn bare_host(input: &str) -> String {
    let s = input.trim();
    let s = s.split("://").next_back().unwrap_or(s);
    let s = s.split(['/', '?', '#']).next().unwrap_or(s);
    let s = s.rsplit('@').next().unwrap_or(s);
    let s = s.split(':').next().unwrap_or(s);
    let s = s.strip_prefix("www.").unwrap_or(s);
    s.to_ascii_lowercase()
}

fn urlencode(s: &str) -> String {
    // Minimal query-component encoding (enough for keys and https URLs).
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::bare_host;

    #[test]
    fn strips_scheme_www_path_and_port() {
        assert_eq!(
            bare_host("https://seo-acquisition.com"),
            "seo-acquisition.com"
        );
        assert_eq!(bare_host("http://www.Example.com/path?q=1"), "example.com");
        assert_eq!(bare_host("example.com"), "example.com");
        assert_eq!(
            bare_host("https://sub.example.com:8443/x"),
            "sub.example.com"
        );
        assert_eq!(bare_host("  https://user@Example.COM/  "), "example.com");
    }
}
