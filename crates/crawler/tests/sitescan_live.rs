//! Site-scan integration tests against a local wiremock server. Like the fetch
//! tests, the mock listens on 127.0.0.1, which the SSRF guard blocks by default,
//! so these set `allow_private_addresses` (the test-only escape hatch) to reach
//! the loopback mock.

use crawler::{CrawlerConfig, FetchOutcome, fetch, scan_site};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn test_cfg() -> CrawlerConfig {
    CrawlerConfig {
        allow_private_addresses: true,
        ..CrawlerConfig::default()
    }
}

fn html(body: &str) -> ResponseTemplate {
    ResponseTemplate::new(200)
        .insert_header("content-type", "text/html")
        .set_body_string(body)
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

    // DIAGNOSTIC: fetch the root directly to see what the home request returns.
    let diag = match fetch(&test_cfg(), &base).await {
        FetchOutcome::Response(r) => format!(
            "status={} ct={:?} final={} bodylen={} bodyhead={:?}",
            r.status,
            r.content_type,
            r.final_url,
            r.body.len(),
            r.body.chars().take(60).collect::<String>()
        ),
        other => format!("FAILED: {other:?}"),
    };

    let scan = scan_site(&test_cfg(), &base, 10).await;

    // P0 + 2 deduped P1 pages (the #section link is the same page as /guide).
    let texts: Vec<&str> = scan.h1s.iter().map(|h| h.text.as_str()).collect();
    assert_eq!(
        scan.pages_scanned, 3,
        "home + 2 unique same-site pages; base={base}; pages={}; h1s={texts:?}; HOME[{diag}]",
        scan.pages_scanned
    );
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
