//! Stripe webhook: signature verification + idempotent receipt (§39, S5).
//! Gated on `DATABASE_URL`.

use api::{ApiConfig, AppState};
use auth::Argon2Params;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use hmac::digest::KeyInit;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use tower::ServiceExt;
use uuid::Uuid;

const SECRET: &str = "whsec_test_secret";

async fn app() -> Option<axum::Router> {
    let url = std::env::var("DATABASE_URL")
        .ok()
        .filter(|s| !s.is_empty())?;
    let db = db::build_pool(&db::DbConfig::worker(&url, 4, None))
        .await
        .ok()?;
    db::migrate::run(&db.pool).await.ok()?;
    let cfg = ApiConfig {
        argon: Argon2Params::insecure_fast(),
        session_ttl_hours: 1,
        secure_cookies: false,
        stripe_webhook_secret: Some(SECRET.to_string()),
        allow_private_fetch: false,
        campaigns_beta: false,
        registration_open: true,
        linkavista_api_key: None,
        linkavista_sandbox: true,
        linkavista_base_url: None,
    };
    Some(api::app(AppState::new(db.pool, cfg)))
}

fn sign(body: &str) -> String {
    let ts = chrono::Utc::now().timestamp();
    let mut mac = Hmac::<Sha256>::new_from_slice(SECRET.as_bytes()).unwrap();
    mac.update(format!("{ts}.{body}").as_bytes());
    let hex: String = mac
        .finalize()
        .into_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    format!("t={ts},v1={hex}")
}

fn post(body: &str, sig: &str) -> Request<Body> {
    Request::post("/v1/billing/webhook")
        .header(header::CONTENT_TYPE, "application/json")
        .header("stripe-signature", sig)
        .body(Body::from(body.to_string()))
        .unwrap()
}

#[tokio::test]
async fn webhook_verifies_signature_and_is_idempotent() {
    let Some(app) = app().await else { return };
    let body = format!(
        r#"{{"id":"evt_{}","type":"invoice.paid"}}"#,
        Uuid::now_v7().simple()
    );
    let sig = sign(&body);

    // Valid signature -> 200, recorded.
    let r1 = app.clone().oneshot(post(&body, &sig)).await.unwrap();
    assert_eq!(r1.status(), StatusCode::OK, "valid webhook accepted");

    // Replay the exact same event -> still 200 (idempotent, no double effect).
    let r2 = app.clone().oneshot(post(&body, &sig)).await.unwrap();
    assert_eq!(r2.status(), StatusCode::OK, "replay is a no-op 200");

    // Tampered body with the old signature -> 401.
    let bad = app
        .oneshot(post(r#"{"id":"evt_evil","type":"invoice.paid"}"#, &sig))
        .await
        .unwrap();
    assert_eq!(
        bad.status(),
        StatusCode::UNAUTHORIZED,
        "bad signature rejected"
    );
}
