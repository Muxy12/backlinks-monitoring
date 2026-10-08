//! Public backlink-checker tool (§50). No DB and no auth: it fetches a mock
//! source page and reports whether it links to the target. The crawler's
//! loopback escape hatch is enabled via ApiConfig so the test can reach the
//! wiremock server on 127.0.0.1.

use api::security::CheckerLimiter;
use api::{ApiConfig, AppState};
use auth::Argon2Params;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use std::time::Duration;
use tower::ServiceExt;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn lazy_pool() -> db::sqlx::PgPool {
    db::sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://unused:unused@127.0.0.1:1/unused")
        .unwrap()
}

fn checker_cfg() -> ApiConfig {
    ApiConfig {
        argon: Argon2Params::insecure_fast(),
        session_ttl_hours: 1,
        secure_cookies: false,
        stripe_webhook_secret: None,
        allow_private_fetch: true,
        campaigns_beta: false, // reach the wiremock loopback server
        registration_open: true,
    }
}

// The checker needs no database; a lazily-connected pool is never queried, so a
// bogus URL is fine and the test runs without DATABASE_URL.
async fn app() -> axum::Router {
    api::app(AppState::new(lazy_pool(), checker_cfg()))
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
        campaigns_beta: false,
        registration_open: true,
    };
    let app = api::app(AppState::new(pool, cfg));

    let (st, v) = check(&app, "http://127.0.0.1:80/x", "https://acme.com/product").await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(v["found"], false);
    assert_eq!(v["error"], "blocked", "loopback is SSRF-blocked: {v}");
}

async fn check_from(
    app: &axum::Router,
    ip: &str,
    source: &str,
    target: &str,
) -> (StatusCode, serde_json::Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::post("/v1/tools/backlink-check")
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-forwarded-for", ip)
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
async fn checker_is_rate_limited_per_ip() {
    // Tiny budget: 2 requests/window/IP so the test is quick and deterministic.
    let limiter = CheckerLimiter::new(2, Duration::from_secs(3600), 8);
    let app = api::app(AppState::with_checker(lazy_pool(), checker_cfg(), limiter));

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/empty"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(b"<html><body>nothing</body></html>".to_vec(), "text/html"),
        )
        .mount(&server)
        .await;
    let src = format!("{}/empty", server.uri());
    let tgt = "https://acme.com/product";

    // Same IP: first two admitted, third rejected with 429.
    let (s1, _) = check_from(&app, "1.2.3.4", &src, tgt).await;
    let (s2, _) = check_from(&app, "1.2.3.4", &src, tgt).await;
    let (s3, v3) = check_from(&app, "1.2.3.4", &src, tgt).await;
    assert_eq!(s1, StatusCode::OK, "1st admitted");
    assert_eq!(s2, StatusCode::OK, "2nd admitted");
    assert_eq!(s3, StatusCode::TOO_MANY_REQUESTS, "3rd over the cap: {v3}");

    // A different IP has its own budget.
    let (s4, _) = check_from(&app, "5.6.7.8", &src, tgt).await;
    assert_eq!(s4, StatusCode::OK, "different IP not affected");
}

#[tokio::test]
async fn responses_carry_security_headers() {
    let app = app().await;
    // /health needs no DB.
    let resp = app
        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let h = resp.headers();
    assert_eq!(h.get("x-content-type-options").unwrap(), "nosniff");
    assert_eq!(h.get("x-frame-options").unwrap(), "DENY");
    assert!(h.get("strict-transport-security").is_some());
    assert!(h.get("referrer-policy").is_some());
}
