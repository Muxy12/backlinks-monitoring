//! Notifications feed (§37): list + unread badge + idempotent mark-read, all
//! tenant-scoped. Gated on `DATABASE_URL`. A notification row is inserted
//! directly (the warranty sweep is the real writer, exercised in the worker
//! crate) so this test focuses on the read/ack API.

use api::{ApiConfig, AppState};
use auth::Argon2Params;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

async fn pool() -> Option<db::sqlx::PgPool> {
    let url = std::env::var("DATABASE_URL")
        .ok()
        .filter(|s| !s.is_empty())?;
    let db = db::build_pool(&db::DbConfig::worker(&url, 4, None))
        .await
        .ok()?;
    db::migrate::run(&db.pool).await.ok()?;
    Some(db.pool)
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

struct Session {
    app: axum::Router,
    pool: db::sqlx::PgPool,
    cookie: String,
    csrf: String,
    org: Uuid,
}

async fn bootstrap() -> Option<Session> {
    let pool = pool().await?;
    let cfg = ApiConfig {
        argon: Argon2Params::insecure_fast(),
        session_ttl_hours: 1,
        secure_cookies: false,
        stripe_webhook_secret: None,
        allow_private_fetch: false,
        campaigns_beta: false,
        registration_open: true,
        linkavista_api_key: None,
        linkavista_sandbox: true,
        linkavista_base_url: None,
        stripe_tax_enabled: false,
        spot_finder: false,
    };
    let app = api::app(AppState::new(pool.clone(), cfg));
    let email = format!("notif-{}@example.com", Uuid::now_v7().simple());
    app.clone()
        .oneshot(
            Request::post("/v1/auth/register")
                .header(header::CONTENT_TYPE, "application/json")
                .body(jbody(
                    serde_json::json!({"email":email,"password":"correct horse","name":"U"}),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let resp = app
        .clone()
        .oneshot(
            Request::post("/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(jbody(
                    serde_json::json!({"email":email,"password":"correct horse"}),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let cookies: Vec<String> = resp
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok().map(String::from))
        .collect();
    let pair = |n: &str| {
        cookies
            .iter()
            .find(|c| c.starts_with(n))
            .map(|c| c.split(';').next().unwrap().to_string())
            .unwrap()
    };
    let session = pair("session=");
    let csrf_pair = pair("csrf=");
    let csrf = csrf_pair.trim_start_matches("csrf=").to_string();
    let cookie = format!("{session}; {csrf_pair}");

    let resp = app
        .clone()
        .oneshot(
            Request::post("/v1/organizations")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .header("x-csrf-token", &csrf)
                .body(jbody(serde_json::json!({
                    "name":"Acme","slug":format!("notif-{}", Uuid::now_v7().simple())
                })))
                .unwrap(),
        )
        .await
        .unwrap();
    let (_, v) = jout(resp).await;
    let org = Uuid::parse_str(v["id"].as_str().unwrap()).unwrap();
    Some(Session {
        app,
        pool,
        cookie,
        csrf,
        org,
    })
}

#[tokio::test]
async fn notifications_list_badge_and_mark_read() {
    let Some(s) = bootstrap().await else { return };
    let org = s.org;

    // Empty to start.
    let resp = s
        .app
        .clone()
        .oneshot(
            Request::get(format!("/v1/organizations/{org}/notifications"))
                .header(header::COOKIE, &s.cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let (st, v) = jout(resp).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(v["unread"], 0);
    assert_eq!(v["items"].as_array().unwrap().len(), 0);

    // Insert one notification for this org directly.
    let notif_id = Uuid::now_v7();
    db::sqlx::query(
        "INSERT INTO notifications (id, organization_id, kind, title, body) \
         VALUES ($1, $2, 'warranty_expiring', 'Warranty expiring soon', 'A link lapses in 5 days.')",
    )
    .bind(notif_id)
    .bind(org)
    .execute(&s.pool)
    .await
    .unwrap();

    // Now listed and counted as unread.
    let resp = s
        .app
        .clone()
        .oneshot(
            Request::get(format!("/v1/organizations/{org}/notifications"))
                .header(header::COOKIE, &s.cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let (st, v) = jout(resp).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(v["unread"], 1);
    let items = v["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["kind"], "warranty_expiring");
    assert_eq!(items[0]["read"], false);

    // Mark read requires CSRF: without it -> 403.
    let resp = s
        .app
        .clone()
        .oneshot(
            Request::post(format!(
                "/v1/organizations/{org}/notifications/{notif_id}/read"
            ))
            .header(header::COOKIE, &s.cookie)
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // With CSRF -> 204, and idempotent on a second call.
    for _ in 0..2 {
        let resp = s
            .app
            .clone()
            .oneshot(
                Request::post(format!(
                    "/v1/organizations/{org}/notifications/{notif_id}/read"
                ))
                .header(header::COOKIE, &s.cookie)
                .header("x-csrf-token", &s.csrf)
                .body(Body::empty())
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    // Unread now 0, and the item shows read.
    let resp = s
        .app
        .clone()
        .oneshot(
            Request::get(format!("/v1/organizations/{org}/notifications"))
                .header(header::COOKIE, &s.cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let (_, v) = jout(resp).await;
    assert_eq!(v["unread"], 0);
    assert_eq!(v["items"][0]["read"], true);

    // Unknown id -> 404.
    let ghost = Uuid::now_v7();
    let resp = s
        .app
        .clone()
        .oneshot(
            Request::post(format!(
                "/v1/organizations/{org}/notifications/{ghost}/read"
            ))
            .header(header::COOKIE, &s.cookie)
            .header("x-csrf-token", &s.csrf)
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
