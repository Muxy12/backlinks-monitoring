//! Backlink CRUD flow over the real routes (Phase 2): create org + project +
//! backlink, verify normalization and dedup. Gated on `DATABASE_URL`.

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
            campaigns_beta: false,
            registration_open: true,
            linkavista_api_key: None,
            linkavista_sandbox: true,
            linkavista_base_url: None,
            stripe_tax_enabled: false,
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
    let email = format!("u-{}@example.com", Uuid::now_v7().simple());
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
                .body(jbody(serde_json::json!({"name":"Acme"})))
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
async fn backlink_create_list_get_and_dedup() {
    let Some(s) = bootstrap().await else { return };
    let org = s.org;

    // Create a project.
    let resp = s
        .app
        .clone()
        .oneshot(
            Request::post(format!("/v1/organizations/{org}/projects"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &s.cookie)
                .header("x-csrf-token", &s.csrf)
                .body(jbody(
                    serde_json::json!({"name":"Site","domain":"example.com"}),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let (st, v) = jout(resp).await;
    assert_eq!(st, StatusCode::CREATED, "project: {v}");
    let project_id = v["id"].as_str().unwrap().to_string();

    // Create a backlink (note http + www — normalization should derive the domain).
    let make = |extra: serde_json::Value| {
        let mut body = serde_json::json!({
            "project_id": project_id,
            "source_url": "http://www.publisher.com/article?utm_source=x",
            "target_url": "https://example.com/landing",
            "expected_anchor_text": "seo tools",
            "expected_rel_type": "dofollow"
        });
        for (k, val) in extra.as_object().unwrap() {
            body[k] = val.clone();
        }
        body
    };
    let resp = s
        .app
        .clone()
        .oneshot(
            Request::post(format!("/v1/organizations/{org}/backlinks"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &s.cookie)
                .header("x-csrf-token", &s.csrf)
                .body(jbody(make(
                    serde_json::json!({"cost": 280.0, "currency": "EUR"}),
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    let (st, v) = jout(resp).await;
    assert_eq!(st, StatusCode::CREATED, "backlink: {v}");
    assert_eq!(v["status"], "pending");
    assert_eq!(
        v["source_domain"], "publisher.com",
        "www stripped in the derived domain"
    );
    assert_eq!(v["target_domain"], "example.com");
    assert_eq!(v["cost"], "280.00");
    let backlink_id = v["id"].as_str().unwrap().to_string();

    // Duplicate (same normalized source+target in the same project) -> 409.
    let resp = s
        .app
        .clone()
        .oneshot(
            Request::post(format!("/v1/organizations/{org}/backlinks"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &s.cookie)
                .header("x-csrf-token", &s.csrf)
                .body(jbody(make(serde_json::json!({}))))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::CONFLICT,
        "duplicate backlink must conflict (§38)"
    );

    // Missing CSRF -> 403.
    let resp = s
        .app
        .clone()
        .oneshot(
            Request::post(format!("/v1/organizations/{org}/backlinks"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &s.cookie)
                .body(jbody(make(serde_json::json!({}))))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // List for the project -> 1.
    let resp = s
        .app
        .clone()
        .oneshot(
            Request::get(format!(
                "/v1/organizations/{org}/backlinks?project_id={project_id}"
            ))
            .header(header::COOKIE, &s.cookie)
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    let (st, v) = jout(resp).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(v.as_array().unwrap().len(), 1);

    // Check now -> 202, and pressing again is idempotent (still 202).
    for _ in 0..3 {
        let resp = s
            .app
            .clone()
            .oneshot(
                Request::post(format!(
                    "/v1/organizations/{org}/backlinks/{backlink_id}/check"
                ))
                .header(header::COOKIE, &s.cookie)
                .header("x-csrf-token", &s.csrf)
                .body(Body::empty())
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::ACCEPTED, "check-now returns 202");
    }

    // Get by id.
    let resp = s
        .app
        .oneshot(
            Request::get(format!("/v1/organizations/{org}/backlinks/{backlink_id}"))
                .header(header::COOKIE, &s.cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
