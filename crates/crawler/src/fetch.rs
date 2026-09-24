//! SSRF-safe HTTP fetch (docs/06 §2). The security-critical orchestration around
//! the pure guard in `backlio_core::net`:
//!
//! 1. validate scheme / credentials / port / host name;
//! 2. resolve the host to IPs and validate **every** one against the guard;
//! 3. **pin** the connection to a validated IP (reqwest `.resolve`), so DNS
//!    rebinding between check and connect cannot reach a private address;
//! 4. drive redirects ourselves (`redirect::none`) and re-run 1–3 on every hop
//!    — reqwest's own redirect following would connect to the next hop without
//!    the guard;
//! 5. stream the body under a hard byte cap.

use crate::config::CrawlerConfig;
use backlio_core::{SsrfBlock, classify_host, classify_ip, classify_port, classify_scheme};
use std::net::SocketAddr;
use std::time::Instant;
use url::Url;

/// Why a fetch could not proceed or complete. Mirrors the distinctions the state
/// machine needs (docs/08 §1): a block/timeout is "could not verify", not "gone".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchError {
    Blocked(SsrfBlock),
    /// Response body exceeded `max_response_bytes` (docs/06 §2.5).
    Oversized,
    Timeout,
    TooManyRedirects,
    Connect(String),
    BadUrl,
}

/// A successful fetch of an HTML-ish page.
#[derive(Debug, Clone)]
pub struct FetchOk {
    pub status: u16,
    pub final_url: String,
    pub redirect_chain: Vec<String>,
    pub body: String,
    pub content_type: Option<String>,
    pub bytes: usize,
    pub duration_ms: u32,
    /// `X-Robots-Tag: noindex` seen on any hop's headers.
    pub header_noindex: bool,
}

/// The result of one fetch: an HTTP response (any status), or a failure.
#[derive(Debug, Clone)]
pub enum FetchOutcome {
    Response(FetchOk),
    Failed(FetchError),
}

/// Validate a single URL's scheme/credentials/port/host (steps 1). Returns the
/// parsed URL, host, and port on success.
fn validate_url(raw: &str) -> Result<(Url, String, u16), FetchError> {
    let url = Url::parse(raw).map_err(|_| FetchError::BadUrl)?;
    let has_credentials = !url.username().is_empty() || url.password().is_some();
    classify_scheme(url.scheme(), has_credentials).map_err(FetchError::Blocked)?;
    let host = url.host_str().ok_or(FetchError::BadUrl)?.to_string();
    classify_host(&host).map_err(FetchError::Blocked)?;
    let port = url.port_or_known_default().ok_or(FetchError::BadUrl)?;
    classify_port(url.port()).map_err(FetchError::Blocked)?;
    Ok((url, host, port))
}

/// Resolve `host:port` and return one validated, connectable address. Every
/// resolved address must pass the guard (unless the test escape hatch is on).
async fn resolve_and_validate(
    host: &str,
    port: u16,
    allow_private: bool,
) -> Result<SocketAddr, FetchError> {
    let addrs: Vec<SocketAddr> = tokio::net::lookup_host((host, port))
        .await
        .map_err(|e| FetchError::Connect(e.to_string()))?
        .collect();
    if addrs.is_empty() {
        return Err(FetchError::Connect("no addresses".into()));
    }
    if !allow_private {
        for a in &addrs {
            classify_ip(a.ip()).map_err(FetchError::Blocked)?;
        }
    }
    Ok(addrs[0])
}

/// Fetch a URL, following redirects safely. Returns the response (any HTTP
/// status is a `Response`) or a `Failed` outcome.
pub async fn fetch(cfg: &CrawlerConfig, start_url: &str) -> FetchOutcome {
    match tokio::time::timeout(cfg.total_timeout, fetch_inner(cfg, start_url)).await {
        Ok(Ok(ok)) => FetchOutcome::Response(ok),
        Ok(Err(e)) => FetchOutcome::Failed(e),
        Err(_) => FetchOutcome::Failed(FetchError::Timeout),
    }
}

async fn fetch_inner(cfg: &CrawlerConfig, start_url: &str) -> Result<FetchOk, FetchError> {
    let started = Instant::now();
    let mut chain: Vec<String> = Vec::new();
    let mut current = start_url.to_string();
    let mut header_noindex = false;

    for _hop in 0..=cfg.max_redirects {
        let (url, host, port) = validate_url(&current)?;
        let addr = resolve_and_validate(&host, port, cfg.allow_private_addresses).await?;

        // Pin the validated IP for this host; drive redirects ourselves.
        let client = reqwest::Client::builder()
            .user_agent(cfg.user_agent.clone())
            .redirect(reqwest::redirect::Policy::none())
            .timeout(cfg.request_timeout)
            .resolve(&host, SocketAddr::new(addr.ip(), port))
            .build()
            .map_err(|e| FetchError::Connect(e.to_string()))?;

        let resp = client
            .get(url.as_str())
            .send()
            .await
            .map_err(|e| classify_reqwest_error(&e))?;

        let status = resp.status();
        if let Some(v) = resp.headers().get("x-robots-tag")
            && v.to_str()
                .map(|s| s.to_ascii_lowercase().contains("noindex"))
                .unwrap_or(false)
        {
            header_noindex = true;
        }

        // Redirect: re-validate the next hop and loop.
        if status.is_redirection() {
            let location = resp
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|v| v.to_str().ok())
                .ok_or(FetchError::BadUrl)?;
            let next = url.join(location).map_err(|_| FetchError::BadUrl)?;
            chain.push(current.clone());
            current = next.to_string();
            continue;
        }

        // Terminal response: capture content type, then stream the body capped.
        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let final_url = resp.url().to_string();

        let mut resp = resp;
        let mut body: Vec<u8> = Vec::new();
        loop {
            match resp.chunk().await {
                Ok(Some(chunk)) => {
                    if body.len() + chunk.len() > cfg.max_response_bytes {
                        return Err(FetchError::Oversized);
                    }
                    body.extend_from_slice(&chunk);
                }
                Ok(None) => break,
                Err(e) => return Err(classify_reqwest_error(&e)),
            }
        }

        return Ok(FetchOk {
            status: status.as_u16(),
            final_url,
            redirect_chain: chain,
            body: String::from_utf8_lossy(&body).into_owned(),
            content_type,
            bytes: body.len(),
            duration_ms: started.elapsed().as_millis() as u32,
            header_noindex,
        });
    }

    Err(FetchError::TooManyRedirects)
}

fn classify_reqwest_error(e: &reqwest::Error) -> FetchError {
    if e.is_timeout() {
        FetchError::Timeout
    } else {
        FetchError::Connect(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn blocks_non_http_scheme_before_any_network() {
        let cfg = CrawlerConfig::default();
        assert!(matches!(
            fetch(&cfg, "file:///etc/passwd").await,
            FetchOutcome::Failed(FetchError::Blocked(_))
        ));
    }

    #[tokio::test]
    async fn blocks_credentials_in_url() {
        let cfg = CrawlerConfig::default();
        assert!(matches!(
            fetch(&cfg, "http://user:pass@example.com/").await,
            FetchOutcome::Failed(FetchError::Blocked(_))
        ));
    }

    #[tokio::test]
    async fn blocks_localhost_by_default() {
        // allow_private is false: 127.0.0.1 must be blocked even though it resolves.
        // Port 80 (allowed) so the block comes from the loopback IP, not the port.
        let cfg = CrawlerConfig::default();
        let out = fetch(&cfg, "http://127.0.0.1/").await;
        assert!(
            matches!(
                out,
                FetchOutcome::Failed(FetchError::Blocked(SsrfBlock::Loopback))
            ),
            "got {out:?}"
        );
    }

    #[tokio::test]
    async fn blocks_non_allowlisted_port() {
        let cfg = CrawlerConfig::default();
        assert!(matches!(
            fetch(&cfg, "http://example.com:22/").await,
            FetchOutcome::Failed(FetchError::Blocked(SsrfBlock::BadPort))
        ));
    }
}
