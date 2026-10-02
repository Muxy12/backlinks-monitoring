//! Crawler fetch configuration (docs/06 §2.5, §5, docs/10).

use std::time::Duration;

#[derive(Debug, Clone)]
pub struct CrawlerConfig {
    pub user_agent: String,
    /// Per-request timeout (one hop).
    pub request_timeout: Duration,
    /// Total wall-clock budget across all redirect hops (docs/06 §2.5).
    pub total_timeout: Duration,
    pub max_redirects: usize,
    /// Hard body cap; a larger response is a memory DoS, not a page (docs/06 §2.5).
    pub max_response_bytes: usize,
    /// **Test-only escape hatch.** When true, the SSRF IP guard is bypassed so a
    /// local mock server (127.0.0.1) can be reached. NEVER true in production —
    /// it disables the product's most important control (docs/06 §2, docs/10 §1).
    pub allow_private_addresses: bool,
    /// When true, a *found* link also triggers a fetch of its target URL (via the
    /// same SSRF-safe `fetch`) to record the target's final URL after redirects
    /// and its HTTP status. Off by default so unit/integration tests and the
    /// public single-check tool never make an extra, network-dependent request;
    /// the production monitoring path turns it on.
    pub check_target: bool,
}

impl Default for CrawlerConfig {
    fn default() -> Self {
        CrawlerConfig {
            user_agent: "LinkPulz/1.0 (+https://linkpulz.com/bot)".into(),
            request_timeout: Duration::from_secs(15),
            total_timeout: Duration::from_secs(30),
            max_redirects: 5,
            max_response_bytes: 2 * 1024 * 1024,
            allow_private_addresses: false,
            check_target: false,
        }
    }
}
