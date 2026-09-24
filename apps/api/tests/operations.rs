//! Campaigns + publishers with rollups (Phase 4, §22/§31): create, attach
//! backlinks, and verify the read-time aggregates — status counts, spend by
//! currency, removal rate, and average lifespan. Gated on `DATABASE_URL`.

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
    };
    let app = api::app(AppState::new(pool.clone(), cfg));
    let email = format!("ops-{}@example.com", Uuid::now_v7().simple());
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
                // Unique slug: the org slug is globally unique and the CI DB is
                // shared across integration binaries (see imports.rs).
                .body(jbody(serde_json::json!({
                    "name":"Acme","slug":format!("ops-{}", Uuid::now_v7().simple())
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

impl Session {
    async fn post(&self, path: String, body: serde_json::Value) -> (StatusCode, serde_json::Value) {
        let resp = self
            .app
            .clone()
            .oneshot(
                Request::post(path)
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::COOKIE, &self.cookie)
                    .header("x-csrf-token", &self.csrf)
                    .body(jbody(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        jout(resp).await
    }
    async fn get(&self, path: String) -> (StatusCode, serde_json::Value) {
        let resp = self
            .app
            .clone()
            .oneshot(
                Request::get(path)
                    .header(header::COOKIE, &self.cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        jout(resp).await
    }
}

#[tokio::test]
async fn campaign_and_publisher_rollups() {
    let Some(s) = bootstrap().await else { return };
    let org = s.org;

    let (st, v) = s
        .post(
            format!("/v1/organizations/{org}/projects"),
            serde_json::json!({"name":"Site","domain":"example.com"}),
        )
        .await;
    assert_eq!(st, StatusCode::CREATED, "project: {v}");
    let project_id = v["id"].as_str().unwrap().to_string();

    // Campaign with a target of 5.
    let (st, v) = s
        .post(
            format!("/v1/organizations/{org}/campaigns"),
            serde_json::json!({
                "project_id": project_id, "name": "Q3 push",
                "target_backlinks": 5, "budget": 1000.0, "currency": "USD"
            }),
        )
        .await;
    assert_eq!(st, StatusCode::CREATED, "campaign: {v}");
    assert_eq!(v["budget"], "1000.00");
    let campaign_id = v["id"].as_str().unwrap().to_string();

    // Publisher.
    let (st, v) = s
        .post(
            format!("/v1/organizations/{org}/publishers"),
            serde_json::json!({"name":"Publisher Co","domain":"publisher.com"}),
        )
        .await;
    assert_eq!(st, StatusCode::CREATED, "publisher: {v}");
    let publisher_id = v["id"].as_str().unwrap().to_string();

    // Two backlinks, both attached to the campaign + publisher, same currency.
    let mut backlink_ids = Vec::new();
    for (i, cost) in [280.0_f64, 120.0_f64].into_iter().enumerate() {
        let (st, v) = s
            .post(
                format!("/v1/organizations/{org}/backlinks"),
                serde_json::json!({
                    "project_id": project_id,
                    "campaign_id": campaign_id,
                    "publisher_id": publisher_id,
                    "source_url": format!("https://publisher.com/article-{i}"),
                    "target_url": "https://example.com/landing",
                    "expected_rel_type": "dofollow",
                    "cost": cost, "currency": "EUR"
                }),
            )
            .await;
        assert_eq!(st, StatusCode::CREATED, "backlink {i}: {v}");
        backlink_ids.push(v["id"].as_str().unwrap().to_string());
    }

    // Campaign rollup while both are still pending.
    let (st, v) = s
        .get(format!(
            "/v1/organizations/{org}/campaigns/{campaign_id}/rollup"
        ))
        .await;
    assert_eq!(st, StatusCode::OK, "campaign rollup: {v}");
    assert_eq!(v["total"], 2);
    assert_eq!(v["live"], 0);
    assert_eq!(v["lost"], 0);
    assert_eq!(v["removal_rate"], 0.0);
    assert_eq!(v["target_backlinks"], 5);
    assert_eq!(v["target_progress"], 0.0);
    // Spend: a single EUR line summing 280 + 120 = 400, never merged with USD.
    let spend = v["spend"].as_array().unwrap();
    assert_eq!(spend.len(), 1, "one currency line: {v}");
    assert_eq!(spend[0]["currency"], "EUR");
    assert_eq!(spend[0]["amount"], "400.00");

    // Simulate one link going lost after ~10 days live, the other staying live.
    // (The state machine sets these in production; here we set them directly to
    // exercise the lifespan/removal-rate math deterministically.)
    db::sqlx::query(
        "UPDATE backlinks SET status = 'lost', \
             last_seen_at = created_at + interval '10 days' \
         WHERE id = $1 AND organization_id = $2",
    )
    .bind(Uuid::parse_str(&backlink_ids[0]).unwrap())
    .bind(org)
    .execute(&s.pool)
    .await
    .unwrap();
    db::sqlx::query("UPDATE backlinks SET status = 'live' WHERE id = $1 AND organization_id = $2")
        .bind(Uuid::parse_str(&backlink_ids[1]).unwrap())
        .bind(org)
        .execute(&s.pool)
        .await
        .unwrap();

    // Publisher rollup now reflects 1 live / 1 lost.
    let (st, v) = s
        .get(format!(
            "/v1/organizations/{org}/publishers/{publisher_id}/rollup"
        ))
        .await;
    assert_eq!(st, StatusCode::OK, "publisher rollup: {v}");
    assert_eq!(v["total"], 2);
    assert_eq!(v["live"], 1);
    assert_eq!(v["lost"], 1);
    assert_eq!(v["removal_rate"], 0.5, "1 of 2 removed");
    let lifespan = v["avg_lifespan_days"].as_f64().unwrap();
    assert!(
        (lifespan - 10.0).abs() < 0.01,
        "avg lifespan ~10 days, got {lifespan}"
    );

    // Campaign rollup also updates: target progress = 1 live / 5 = 0.2.
    let (st, v) = s
        .get(format!(
            "/v1/organizations/{org}/campaigns/{campaign_id}/rollup"
        ))
        .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(v["live"], 1);
    assert_eq!(v["target_progress"], 0.2);

    // A random campaign/publisher id under our own org -> 404 (not in org).
    let ghost = Uuid::now_v7();
    let (st, _) = s
        .get(format!("/v1/organizations/{org}/campaigns/{ghost}/rollup"))
        .await;
    assert_eq!(st, StatusCode::NOT_FOUND, "unknown campaign rollup is 404");
    let (st, _) = s
        .get(format!("/v1/organizations/{org}/publishers/{ghost}/rollup"))
        .await;
    assert_eq!(st, StatusCode::NOT_FOUND, "unknown publisher rollup is 404");
}
