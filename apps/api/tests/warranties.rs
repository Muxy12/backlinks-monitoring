//! Warranty tracking over the API (§32): create backlinks with warranties,
//! assert the materialized expiration + computed status, and the
//! expiring-warranties list. Gated on `DATABASE_URL`.

use api::{ApiConfig, AppState};
use auth::Argon2Params;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

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

struct Session {
    app: axum::Router,
    cookie: String,
    csrf: String,
    org: Uuid,
}

async fn bootstrap() -> Option<Session> {
    let app = api::app(state().await?);
    let email = format!("war-{}@example.com", Uuid::now_v7().simple());
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
                    "name":"Acme","slug":format!("war-{}", Uuid::now_v7().simple())
                })))
                .unwrap(),
        )
        .await
        .unwrap();
    let (_, v) = jout(resp).await;
    let org = Uuid::parse_str(v["id"].as_str().unwrap()).unwrap();
    Some(Session {
        app,
        cookie,
        csrf,
        org,
    })
}

#[tokio::test]
async fn warranty_materialization_status_and_expiring_list() {
    let Some(s) = bootstrap().await else { return };
    let org = s.org;

    let post = |path: String, body: serde_json::Value| {
        let app = s.app.clone();
        let cookie = s.cookie.clone();
        let csrf = s.csrf.clone();
        async move {
            let resp = app
                .oneshot(
                    Request::post(path)
                        .header(header::CONTENT_TYPE, "application/json")
                        .header(header::COOKIE, cookie)
                        .header("x-csrf-token", csrf)
                        .body(jbody(body))
                        .unwrap(),
                )
                .await
                .unwrap();
            jout(resp).await
        }
    };

    let (st, v) = post(
        format!("/v1/organizations/{org}/projects"),
        serde_json::json!({"name":"Site","domain":"example.com"}),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "project: {v}");
    let project_id = v["id"].as_str().unwrap().to_string();

    // A 6-month warranty -> active, with a materialized expiration date.
    let (st, v) = post(
        format!("/v1/organizations/{org}/backlinks"),
        serde_json::json!({
            "project_id": project_id,
            "source_url": "https://pub-a.com/post",
            "target_url": "https://example.com/landing",
            "expected_rel_type": "dofollow",
            "warranty_type": "months", "warranty_value": 6
        }),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "warranty create: {v}");
    assert_eq!(v["warranty_type"], "months");
    assert_eq!(v["warranty_value"], 6);
    assert!(
        v["warranty_expiration"].is_string(),
        "expiration materialized: {v}"
    );
    assert_eq!(v["warranty_status"], "active");

    // A 2-day warranty -> expiring soon (inside the 14-day window).
    let (st, v) = post(
        format!("/v1/organizations/{org}/backlinks"),
        serde_json::json!({
            "project_id": project_id,
            "source_url": "https://pub-b.com/post",
            "target_url": "https://example.com/landing",
            "expected_rel_type": "dofollow",
            "warranty_type": "days", "warranty_value": 2
        }),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "short warranty: {v}");
    assert_eq!(v["warranty_status"], "expiring_soon");

    // days/months warranty without a value -> 422.
    let (st, _) = post(
        format!("/v1/organizations/{org}/backlinks"),
        serde_json::json!({
            "project_id": project_id,
            "source_url": "https://pub-c.com/post",
            "target_url": "https://example.com/landing",
            "expected_rel_type": "dofollow",
            "warranty_type": "days"
        }),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::UNPROCESSABLE_ENTITY,
        "missing value rejected"
    );

    // Expiring list (default 14-day window) contains only the 2-day link.
    let resp = s
        .app
        .clone()
        .oneshot(
            Request::get(format!("/v1/organizations/{org}/warranties/expiring"))
                .header(header::COOKIE, &s.cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let (st, v) = jout(resp).await;
    assert_eq!(st, StatusCode::OK);
    let arr = v.as_array().unwrap();
    assert_eq!(arr.len(), 1, "only the 2-day warranty is expiring: {v}");
    assert_eq!(arr[0]["warranty_status"], "expiring_soon");
}
