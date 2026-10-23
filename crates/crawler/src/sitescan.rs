//! Spot Finder site scan (Lot A): crawl a site's homepage (P0) and the pages one
//! click away (P1), collecting each page's `<h1>` headings. This is the
//! keyword-detection seed — the user reviews/edits the result before any paid
//! SERP lookup, so there is no external provider and no cost here. It reuses the
//! same SSRF-safe [`fetch`] as monitoring: every hop is validated, so a page
//! cannot redirect us into a private address.

use crate::config::CrawlerConfig;
use crate::fetch::{FetchOutcome, fetch};
use crate::parse::parse_page;
use std::collections::HashSet;
use url::Url;

/// One `<h1>` found during a scan, with the page it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedH1 {
    pub text: String,
    pub url: String,
}

/// The outcome of a site scan.
#[derive(Debug, Clone, Default)]
pub struct SiteScan {
    /// How many pages were actually fetched (P0 + reachable P1), any status.
    pub pages_scanned: usize,
    /// Headings collected across all scanned HTML pages, in crawl order.
    pub h1s: Vec<ScannedH1>,
}

/// A response is parseable HTML when it says so, or doesn't say otherwise — the
/// parser is lenient, so a missing content-type is treated as HTML.
fn is_html(ct: Option<&str>) -> bool {
    match ct {
        Some(c) => {
            let c = c.to_ascii_lowercase();
            c.contains("text/html") || c.contains("application/xhtml")
        }
        None => true,
    }
}

/// Registrable-ish host for "same site" comparison: lowercased, `www.` stripped.
/// Deliberately simple (no public-suffix list) — P1 is "one click from the home
/// page on the same host", and an exact host match after dropping `www` is the
/// right, conservative rule for Lot A.
fn site_host(u: &Url) -> Option<String> {
    let h = u.host_str()?.to_ascii_lowercase();
    Some(h.strip_prefix("www.").unwrap_or(&h).to_string())
}

/// Dedup key for a page: host (sans `www`) + path, ignoring query, fragment,
/// trailing slash and case — so `/x`, `/x/`, `/x?y` and `/x#z` are one page.
fn canon_key(u: &Url) -> String {
    let host = u.host_str().unwrap_or("").to_ascii_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host);
    let path = u.path().trim_end_matches('/');
    format!("{host}{path}")
}

/// Crawl `home_url` (P0) and up to `max_p1` same-site pages linked from it (P1),
/// collecting their `<h1>` headings. Best-effort: unreachable or non-HTML pages
/// are skipped, never fatal, so a partial result is still useful.
pub async fn scan_site(cfg: &CrawlerConfig, home_url: &str, max_p1: usize) -> SiteScan {
    let max_p1 = max_p1.min(30);
    let mut scan = SiteScan::default();

    // P0 — the homepage.
    let FetchOutcome::Response(home) = fetch(cfg, home_url).await else {
        return scan;
    };
    scan.pages_scanned += 1;
    if !((200..300).contains(&home.status) && is_html(home.content_type.as_deref())) {
        return scan;
    }
    let Ok(base) = Url::parse(&home.final_url) else {
        return scan;
    };
    let Some(home_host) = site_host(&base) else {
        return scan;
    };

    let parsed = parse_page(&home.body);
    // Relative links resolve against <base href> when present, else the final URL.
    let doc_base = parsed
        .base_href
        .as_deref()
        .and_then(|b| base.join(b).ok())
        .unwrap_or_else(|| base.clone());
    for text in parsed.h1s {
        scan.h1s.push(ScannedH1 {
            text,
            url: home.final_url.clone(),
        });
    }

    // Collect same-site P1 URLs from the homepage's links, resolved + deduped.
    let mut seen: HashSet<String> = HashSet::new();
    seen.insert(canon_key(&base));
    let mut p1: Vec<String> = Vec::new();
    for link in parsed.links {
        let Ok(mut abs) = doc_base.join(&link.href_raw) else {
            continue;
        };
        abs.set_fragment(None);
        if !matches!(abs.scheme(), "http" | "https") {
            continue;
        }
        if site_host(&abs).as_deref() != Some(home_host.as_str()) {
            continue;
        }
        if seen.insert(canon_key(&abs)) {
            p1.push(abs.into());
            if p1.len() >= max_p1 {
                break;
            }
        }
    }

    // P1 — pages one click away (sequential; each uses the same SSRF guard).
    for url in p1 {
        if let FetchOutcome::Response(resp) = fetch(cfg, &url).await {
            scan.pages_scanned += 1;
            if (200..300).contains(&resp.status) && is_html(resp.content_type.as_deref()) {
                let p = parse_page(&resp.body);
                for text in p.h1s {
                    scan.h1s.push(ScannedH1 {
                        text,
                        url: resp.final_url.clone(),
                    });
                }
            }
        }
    }

    scan
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn site_host_strips_www_and_lowercases() {
        let u = Url::parse("https://WWW.Example.com/x").unwrap();
        assert_eq!(site_host(&u).as_deref(), Some("example.com"));
    }

    #[test]
    fn canon_key_ignores_trailing_slash_query_and_fragment() {
        let a = Url::parse("https://ex.com/guide/").unwrap();
        let b = Url::parse("https://ex.com/guide?x=1#y").unwrap();
        assert_eq!(canon_key(&a), canon_key(&b));
        assert_eq!(canon_key(&a), "ex.com/guide");
    }

    #[test]
    fn is_html_is_lenient_without_content_type() {
        assert!(is_html(None));
        assert!(is_html(Some("text/html; charset=utf-8")));
        assert!(!is_html(Some("application/pdf")));
    }
}
