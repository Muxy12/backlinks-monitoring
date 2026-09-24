//! Fetch-layer integration tests against a local wiremock server (docs/06 §7
//! network fixtures). The mock listens on 127.0.0.1, which the SSRF guard blocks
//! by default — these tests set `allow_private_addresses` (the documented
//! test-only escape hatch) so the loopback mock is reachable. Real SSRF blocking
//! is covered by `backlio_core::net` unit tests and the fetch.rs unit tests.

use crawler::{CrawlerConfig, FetchError, FetchOutcome, fetch};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn test_cfg() -> CrawlerConfig {
    CrawlerConfig {
        allow_private_addresses: true,
        ..CrawlerConfig::default()
    }
}

#[tokio::test]
async fn fetches_200_html_body() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/article"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/html")
                .set_body_string(
                    "<html><body><a href=\"https://acme.com/p\">buy</a></body></html>",
                ),
        )
        .mount(&server)
        .await;

    let out = fetch(&test_cfg(), &format!("{}/article", server.uri())).await;
    match out {
        FetchOutcome::Response(ok) => {
            assert_eq!(ok.status, 200);
            assert!(ok.body.contains("acme.com/p"));
            assert!(
                ok.content_type
                    .as_deref()
                    .unwrap_or("")
                    .contains("text/html")
            );
            assert!(ok.redirect_chain.is_empty());
        }
        other => panic!("expected 200 response, got {other:?}"),
    }
}

#[tokio::test]
async fn follows_a_redirect_and_records_the_chain() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/old"))
        .respond_with(ResponseTemplate::new(301).insert_header("location", "/new"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/new"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/html")
                .set_body_string("<p>moved</p>"),
        )
        .mount(&server)
        .await;

    let out = fetch(&test_cfg(), &format!("{}/old", server.uri())).await;
    match out {
        FetchOutcome::Response(ok) => {
            assert_eq!(ok.status, 200);
            assert!(ok.final_url.ends_with("/new"));
            assert_eq!(ok.redirect_chain.len(), 1, "one hop recorded");
        }
        other => panic!("expected redirected 200, got {other:?}"),
    }
}

#[tokio::test]
async fn reports_404_as_a_response_not_a_failure() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/missing"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let out = fetch(&test_cfg(), &format!("{}/missing", server.uri())).await;
    match out {
        FetchOutcome::Response(ok) => assert_eq!(ok.status, 404, "404 is an answer, not a failure"),
        other => panic!("expected 404 response, got {other:?}"),
    }
}

#[tokio::test]
async fn oversized_body_is_blocked() {
    let server = MockServer::start().await;
    let big = "x".repeat(50_000);
    Mock::given(method("GET"))
        .and(path("/big"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/html")
                .set_body_string(big),
        )
        .mount(&server)
        .await;

    let cfg = CrawlerConfig {
        max_response_bytes: 10_000,
        ..test_cfg()
    };
    let out = fetch(&cfg, &format!("{}/big", server.uri())).await;
    assert!(
        matches!(out, FetchOutcome::Failed(FetchError::Oversized)),
        "got {out:?}"
    );
}

#[tokio::test]
async fn too_many_redirects_is_failed() {
    let server = MockServer::start().await;
    // /loop -> /loop forever.
    Mock::given(method("GET"))
        .and(path("/loop"))
        .respond_with(ResponseTemplate::new(302).insert_header("location", "/loop"))
        .mount(&server)
        .await;

    let cfg = CrawlerConfig {
        max_redirects: 3,
        ..test_cfg()
    };
    let out = fetch(&cfg, &format!("{}/loop", server.uri())).await;
    assert!(
        matches!(out, FetchOutcome::Failed(FetchError::TooManyRedirects)),
        "got {out:?}"
    );
}
