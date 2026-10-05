//! End-to-end API tests driving the real router with `tower::oneshot` against a
//! real Postgres (docs/13 Phase 1 exit: register → login → create org). Gated on
//! `DATABASE_URL`; skip-and-pass when unset so local `cargo test` is not blocked.

use api::{ApiConfig, AppState};
use auth::Argon2Params;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use tower::ServiceExt; // for `oneshot`
use uuid::Uuid;

async fn state() -> Option<AppState> {
    let url = std::env::var("DATABASE_URL")
        .ok()
        .filter(|s| !s.is_empty())?;
    let db = db::build_pool(&db::DbConfig::worker(&url, 4, None))
        .await
        .ok()?;
    db::migrate::run(&db.pool).await.ok()?;
    // Cheap Argon params + insecure cookies (http test client).
    let cfg = ApiConfig {
        argon: Argon2Params::insecure_fast(),
        session_ttl_hours: 1,
        secure_cookies: false,
        stripe_webhook_secret: None,
        allow_private_fetch: false,
        campaigns_beta: false,
    };
    Some(AppState::new(db.pool, cfg))
}

fn json_body(v: serde_json::Value) -> Body {
    Body::from(v.to_string())
}

async fn read_json(resp: axum::response::Response) -> (StatusCode, serde_json::Value) {
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let val = if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
    };
    (status, val)
}

fn uniq_email() -> String {
    format!("user-{}@example.com", Uuid::now_v7().simple())
}

#[tokio::test]
async fn health_needs_no_auth() {
    let Some(state) = state().await else { return };
    let resp = api::app(state)
        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn register_login_me_and_create_org_flow() {
    let Some(state) = state().await else { return };
    let app = api::app(state);
    let email = uniq_email();

    // ── Register ─────────────────────────────────────────────────────────────
    let resp = app
        .clone()
        .oneshot(
            Request::post("/v1/auth/register")
                .header(header::CONTENT_TYPE, "application/json")
                .body(json_body(serde_json::json!({
                    "email": email, "password": "correct horse", "name": "Alice"
                })))
                .unwrap(),
        )
        .await
        .unwrap();
    let (status, body) = read_json(resp).await;
    assert_eq!(status, StatusCode::CREATED, "register: {body}");
    assert_eq!(body["email"], email);

    // Duplicate register -> 409.
    let resp = app
        .clone()
        .oneshot(
            Request::post("/v1/auth/register")
                .header(header::CONTENT_TYPE, "application/json")
                .body(json_body(serde_json::json!({
                    "email": email, "password": "correct horse", "name": "Alice"
                })))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);

    // ── Login ────────────────────────────────────────────────────────────────
    let resp = app
        .clone()
        .oneshot(
            Request::post("/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(json_body(
                    serde_json::json!({ "email": email, "password": "correct horse" }),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let cookies = collect_cookies(&resp);
    let session_cookie = cookies
        .iter()
        .find(|c| c.starts_with("session="))
        .cloned()
        .expect("session cookie");
    let csrf_cookie = cookies
        .iter()
        .find(|c| c.starts_with("csrf="))
        .cloned()
        .expect("csrf cookie");
    let csrf_value = csrf_cookie
        .trim_start_matches("csrf=")
        .split(';')
        .next()
        .unwrap()
        .to_string();
    let cookie_header = format!(
        "{}; {}",
        first_pair(&session_cookie),
        first_pair(&csrf_cookie)
    );

    // ── Wrong password -> 401 (uniform) ──────────────────────────────────────
    let resp = app
        .clone()
        .oneshot(
            Request::post("/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(json_body(
                    serde_json::json!({ "email": email, "password": "WRONG" }),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // ── /me with the session cookie ──────────────────────────────────────────
    let resp = app
        .clone()
        .oneshot(
            Request::get("/v1/me")
                .header(header::COOKIE, &cookie_header)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let (status, body) = read_json(resp).await;
    assert_eq!(status, StatusCode::OK, "me: {body}");
    assert_eq!(body["user"]["email"], email);
    assert_eq!(body["organizations"].as_array().unwrap().len(), 0);

    // ── /me without auth -> 401 ──────────────────────────────────────────────
    let resp = app
        .clone()
        .oneshot(Request::get("/v1/me").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // ── Create org WITHOUT csrf header -> 403 ────────────────────────────────
    let resp = app
        .clone()
        .oneshot(
            Request::post("/v1/organizations")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie_header)
                .body(json_body(serde_json::json!({ "name": "Acme SEO" })))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "missing CSRF must be rejected"
    );

    // ── Create org WITH csrf header -> 201 ───────────────────────────────────
    let resp = app
        .clone()
        .oneshot(
            Request::post("/v1/organizations")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie_header)
                .header("x-csrf-token", &csrf_value)
                .body(json_body(serde_json::json!({ "name": "Acme SEO" })))
                .unwrap(),
        )
        .await
        .unwrap();
    let (status, body) = read_json(resp).await;
    assert_eq!(status, StatusCode::CREATED, "create org: {body}");
    assert_eq!(body["role"], "owner");
    assert_eq!(body["slug"], "acme-seo");

    // ── /me now lists the org ────────────────────────────────────────────────
    let resp = app
        .oneshot(
            Request::get("/v1/me")
                .header(header::COOKIE, &cookie_header)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let (_, body) = read_json(resp).await;
    assert_eq!(body["organizations"].as_array().unwrap().len(), 1);
    assert_eq!(body["organizations"][0]["role"], "owner");
}

fn collect_cookies(resp: &axum::response::Response) -> Vec<String> {
    resp.headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok().map(String::from))
        .collect()
}

/// The `name=value` pair of a Set-Cookie header, dropping attributes.
fn first_pair(set_cookie: &str) -> &str {
    set_cookie.split(';').next().unwrap()
}
