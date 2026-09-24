//! Public backlink-checker tool (§50). No DB and no auth: it fetches a mock
//! source page and reports whether it links to the target. The crawler's
//! loopback escape hatch is enabled via ApiConfig so the test can reach the
//! wiremock server on 127.0.0.1.

use api::{ApiConfig, AppState};
use auth::Argon2Params;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use tower::ServiceExt;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// The checker needs no database; a lazily-connected pool is never queried, so a
// bogus URL is fine and the test runs without DATABASE_URL.
async fn app() -> axum::Router {
    let pool = db::sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://unused:unused@127.0.0.1:1/unused")
        .unwrap();
    let cfg = ApiConfig {
        argon: Argon2Params::insecure_fast(),
        session_ttl_hours: 1,
        secure_cookies: false,
        stripe_webhook_secret: None,
        allow_private_fetch: true, // reach the wiremock loopback server
    };
    api::app(AppState::new(pool, cfg))
}

fn jbody(v: serde_json::Value) -> Body {
    Body::from(v.to_string())
}
async fn jout(resp: axum::response::Response) -> (StatusCode, serde_json::Value) {
    let s = resp.status();
    let b = resp.into_body().collect().await.unwrap().to_bytes();
    (
        s,
        serde_json::from_slice(&b).unwrap_or(serde_json::Value::Null),
    )
}

async fn check(app: &axum::Router, source: &str, target: &str) -> (StatusCode, serde_json::Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::post("/v1/tools/backlink-check")
                .header(header::CONTENT_TYPE, "application/json")
                .body(jbody(
                    serde_json::json!({"source_url": source, "target_url": target}),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    jout(resp).await
}

#[tokio::test]
async fn checker_reports_present_absent_and_invalid() {
    let app = app().await;

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/article"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            r#"<html><body><a href="https://acme.com/product" rel="nofollow">great tool</a></body></html>"#
                .as_bytes()
                .to_vec(),
            "text/html",
        ))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/empty"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            b"<html><body>nothing here</body></html>".to_vec(),
            "text/html",
        ))
        .mount(&server)
        .await;

    // Present: the page links to the target, nofollow, with the anchor.
    let (st, v) = check(
        &app,
        &format!("{}/article", server.uri()),
        "https://acme.com/product",
    )
    .await;
    assert_eq!(st, StatusCode::OK, "present: {v}");
    assert_eq!(v["found"], true);
    assert_eq!(v["http_status"], 200);
    assert_eq!(v["anchor_text"], "great tool");
    assert_eq!(v["is_dofollow"], false, "rel=nofollow -> not dofollow");
    assert!(
        v["rel"].as_array().unwrap().iter().any(|r| r == "nofollow"),
        "rel includes nofollow: {v}"
    );

    // Absent: page fetched fine, but no link to the target.
    let (st, v) = check(
        &app,
        &format!("{}/empty", server.uri()),
        "https://acme.com/product",
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(v["found"], false);
    assert_eq!(v["http_status"], 200);
    assert!(v["error"].is_null(), "a read page is not an error: {v}");

    // Invalid input -> 422.
    let (st, _) = check(&app, "ftp://nope", "https://acme.com/product").await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn checker_blocks_private_targets_when_not_allowed() {
    // With the default (production) config, a loopback source is SSRF-blocked
    // and reported as such — never fetched.
    let pool = db::sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://unused:unused@127.0.0.1:1/unused")
        .unwrap();
    let cfg = ApiConfig {
        argon: Argon2Params::insecure_fast(),
        session_ttl_hours: 1,
        secure_cookies: false,
        stripe_webhook_secret: None,
        allow_private_fetch: false,
    };
    let app = api::app(AppState::new(pool, cfg));

    let (st, v) = check(&app, "http://127.0.0.1:80/x", "https://acme.com/product").await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(v["found"], false);
    assert_eq!(v["error"], "blocked", "loopback is SSRF-blocked: {v}");
}
