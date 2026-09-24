//! /internal/tick (docs/17): the cron-driven monitoring tick. Auth by
//! SCHEDULER_SECRET; it schedules due checks. Crawling is env-gated (skipped
//! outside production), so this test stays network-free. Gated on `DATABASE_URL`.

use api::{ApiConfig, AppState};
use auth::Argon2Params;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

const SECRET: &str = "tick-secret-xyz";

async fn state() -> Option<AppState> {
    let url = std::env::var("DATABASE_URL")
        .ok()
        .filter(|s| !s.is_empty())?;
    let db = db::build_pool(&db::DbConfig::worker(&url, 4, None))
        .await
        .ok()?;
    db::migrate::run(&db.pool).await.ok()?;
    Some(AppState::new(
        db.pool,
        ApiConfig {
            argon: Argon2Params::insecure_fast(),
            session_ttl_hours: 1,
            secure_cookies: false,
            stripe_webhook_secret: None,
            allow_private_fetch: false,
        },
    ))
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

async fn tick(app: &axum::Router, secret: Option<&str>) -> (StatusCode, serde_json::Value) {
    let mut req = Request::post("/internal/tick");
    if let Some(s) = secret {
        req = req.header("x-scheduler-secret", s);
    }
    jout(
        app.clone()
            .oneshot(req.body(Body::empty()).unwrap())
            .await
            .unwrap(),
    )
    .await
}

#[tokio::test]
async fn internal_tick_auth_and_schedules() {
    // SAFETY: this test's own process; set before the app reads it.
    unsafe {
        std::env::set_var("SCHEDULER_SECRET", SECRET);
    }
    let Some(st) = state().await else { return };
    let app = api::app(st);

    // Seed an org + project + backlink so there is a due check to schedule.
    // (Bootstrap over the API to get a real, scoped backlink.)
    let email = format!("tick-{}@example.com", Uuid::now_v7().simple());
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
    let login = app
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
    let cookies: Vec<String> = login
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok().map(String::from))
        .collect();
    let pick = |n: &str| {
        cookies
            .iter()
            .find(|c| c.starts_with(n))
            .map(|c| c.split(';').next().unwrap().to_string())
            .unwrap()
    };
    let cookie = format!("{}; {}", pick("session="), pick("csrf="));
    let csrf = pick("csrf=").trim_start_matches("csrf=").to_string();
    let post = |path: String, b: serde_json::Value| {
        let app = app.clone();
        let cookie = cookie.clone();
        let csrf = csrf.clone();
        async move {
            let r = app
                .oneshot(
                    Request::post(path)
                        .header(header::CONTENT_TYPE, "application/json")
                        .header(header::COOKIE, cookie)
                        .header("x-csrf-token", csrf)
                        .body(jbody(b))
                        .unwrap(),
                )
                .await
                .unwrap();
            jout(r).await.1
        }
    };
    let org = post(
        "/v1/organizations".into(),
        serde_json::json!({"name":"Acme","slug":format!("tick-{}", Uuid::now_v7().simple())}),
    )
    .await["id"]
        .as_str()
        .unwrap()
        .to_string();
    let project = post(
        format!("/v1/organizations/{org}/projects"),
        serde_json::json!({"name":"Site","domain":"example.com"}),
    )
    .await["id"]
        .as_str()
        .unwrap()
        .to_string();
    post(
        format!("/v1/organizations/{org}/backlinks"),
        serde_json::json!({
            "project_id": project,
            "source_url":"https://pub.example.com/post",
            "target_url":"https://example.com/landing",
            "expected_rel_type":"dofollow"
        }),
    )
    .await;

    // No secret header -> 401.
    assert_eq!(tick(&app, None).await.0, StatusCode::UNAUTHORIZED);
    // Wrong secret -> 401.
    assert_eq!(tick(&app, Some("nope")).await.0, StatusCode::UNAUTHORIZED);

    // Correct secret -> 200; the freshly-created backlink is due, so it schedules.
    let (stt, v) = tick(&app, Some(SECRET)).await;
    assert_eq!(stt, StatusCode::OK, "tick: {v}");
    assert!(
        v["scheduled_backlinks"].as_i64().unwrap() >= 1,
        "a due backlink is scheduled: {v}"
    );
    // Non-production env -> crawling skipped, nothing processed (no network).
    assert_eq!(v["processed"], 0);
}
