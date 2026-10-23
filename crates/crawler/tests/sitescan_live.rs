//! Site-scan integration tests against a local wiremock server. Like the fetch
//! tests, the mock listens on 127.0.0.1, which the SSRF guard blocks by default,
//! so these set `allow_private_addresses` (the test-only escape hatch) to reach
//! the loopback mock.

use crawler::{CrawlerConfig, scan_site};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn test_cfg() -> CrawlerConfig {
    CrawlerConfig {
        allow_private_addresses: true,
        ..CrawlerConfig::default()
    }
}

// `set_body_raw` with an explicit mime — `set_body_string` would force
// `text/plain`, which the scanner (correctly) skips as non-HTML.
fn html(body: &str) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_raw(body.as_bytes().to_vec(), "text/html")
}

#[tokio::test]
async fn collects_h1s_from_home_and_one_click_pages() {
    let server = MockServer::start().await;
    let base = server.uri();

    // Homepage links to /guide and /pricing (same site) plus an external site.
    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(html(
            r#"<html><body>
                 <h1>Backlink monitoring</h1>
                 <a href="/guide">Guide</a>
                 <a href="/pricing">Pricing</a>
                 <a href="https://other-site.com/x">external</a>
                 <a href="/guide#section">Guide again</a>
               </body></html>"#,
        ))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/guide"))
        .respond_with(html("<h1>Link building guide</h1>"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/pricing"))
        .respond_with(html("<h1>Pricing</h1>"))
        .mount(&server)
        .await;
    // External page must never be fetched (same-site filter) — not mounted.

    let scan = scan_site(&test_cfg(), &base, 10).await;

    // P0 + 2 deduped P1 pages (the #section link is the same page as /guide).
    assert_eq!(scan.pages_scanned, 3, "home + 2 unique same-site pages");
    let texts: Vec<&str> = scan.h1s.iter().map(|h| h.text.as_str()).collect();
    assert!(texts.contains(&"Backlink monitoring"));
    assert!(texts.contains(&"Link building guide"));
    assert!(texts.contains(&"Pricing"));
    // Every heading carries the page it came from.
    assert!(scan.h1s.iter().all(|h| h.url.starts_with(&base)));
}

#[tokio::test]
async fn respects_the_p1_cap() {
    let server = MockServer::start().await;
    let base = server.uri();
    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(html(
            r#"<h1>Home</h1>
               <a href="/a">a</a><a href="/b">b</a><a href="/c">c</a>"#,
        ))
        .mount(&server)
        .await;
    for p in ["/a", "/b", "/c"] {
        Mock::given(method("GET"))
            .and(path(p))
            .respond_with(html("<h1>page</h1>"))
            .mount(&server)
            .await;
    }

    // Cap at 1 P1 page: home + exactly one linked page.
    let scan = scan_site(&test_cfg(), &base, 1).await;
    assert_eq!(scan.pages_scanned, 2);
}

#[tokio::test]
async fn unreachable_home_yields_empty_scan() {
    // Nothing mounted: the homepage 404s, so no H1s and it still counts as one
    // page scanned (we reached the server, it just had no usable content).
    let server = MockServer::start().await;
    let scan = scan_site(&test_cfg(), &server.uri(), 10).await;
    assert!(scan.h1s.is_empty());
    assert_eq!(scan.pages_scanned, 1);
}
