//! User feedback (migration 0033): a member submits feature requests and bug
//! reports and lists their own; a non-admin cannot see the backoffice list; the
//! platform admin sees every submission and triages it. Gated on `DATABASE_URL`.

use api::{ApiConfig, AppState};
use auth::Argon2Params;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

fn db_url() -> Option<String> {
    std::env::var("DATABASE_URL").ok().filter(|s| !s.is_empty())
}

async fn state() -> Option<AppState> {
    let url = db_url()?;
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
            spot_finder: false,
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
    email: String,
}

async fn bootstrap() -> Option<Session> {
    let app = api::app(state().await?);
    let email = format!("fb-{}@example.com", Uuid::now_v7().simple());
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
                    "name":"Acme","slug":format!("fb-{}", Uuid::now_v7().simple())
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
        email,
    })
}

impl Session {
    async fn post(&self, path: &str, body: serde_json::Value) -> (StatusCode, serde_json::Value) {
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
    async fn get(&self, path: &str) -> (StatusCode, serde_json::Value) {
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

/// Promote a user to platform admin directly (the backoffice is otherwise
/// invisible). Uses a short-lived pool on the same test database.
async fn make_admin(email: &str) {
    let url = db_url().unwrap();
    let db = db::build_pool(&db::DbConfig::worker(&url, 2, None))
        .await
        .unwrap();
    sqlx::query("UPDATE users SET is_platform_admin = true WHERE email = $1")
        .bind(email)
        .execute(&db.pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn feedback_submit_list_and_triage() {
    let Some(s) = bootstrap().await else {
        return;
    };

    // Submit a feature request (with the current workspace) and a bug report.
    let (st, v) = s
        .post(
            "/v1/feedback",
            serde_json::json!({
                "kind":"feature","title":"Weekly digest",
                "body":"Email me the links I lost each week","organization_id": s.org
            }),
        )
        .await;
    assert_eq!(st, StatusCode::CREATED, "submit feature: {v}");
    assert_eq!(v["kind"], "feature");
    assert_eq!(v["status"], "open");
    let fid = v["id"].as_str().unwrap().to_string();

    let (st, _) = s
        .post(
            "/v1/feedback",
            serde_json::json!({"kind":"bug","title":"Export broken","body":"The button does nothing"}),
        )
        .await;
    assert_eq!(st, StatusCode::CREATED);

    // Bad kind and empty title are rejected (422).
    let (st, _) = s
        .post(
            "/v1/feedback",
            serde_json::json!({"kind":"spam","title":"x","body":"y"}),
        )
        .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    let (st, _) = s
        .post(
            "/v1/feedback",
            serde_json::json!({"kind":"feature","title":"   ","body":"y"}),
        )
        .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);

    // The user sees exactly their own two submissions.
    let (st, v) = s.get("/v1/feedback/mine").await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(v.as_array().unwrap().len(), 2, "two own submissions");

    // The backoffice is invisible to a non-admin (404, not 403).
    let (st, _) = s.get("/v1/admin/feedback").await;
    assert_eq!(st, StatusCode::NOT_FOUND);

    // After promotion, the admin sees every submission (shared test DB -> use
    // >=) and the open count includes ours.
    make_admin(&s.email).await;
    let (st, v) = s.get("/v1/admin/feedback").await;
    assert_eq!(st, StatusCode::OK, "admin list: {v}");
    assert!(v["items"].as_array().unwrap().len() >= 2);
    assert!(v["open"].as_i64().unwrap() >= 2);

    // Triage the feature to "planned".
    let (st, v) = s
        .post(
            &format!("/v1/admin/feedback/{fid}/status"),
            serde_json::json!({"status":"planned"}),
        )
        .await;
    assert_eq!(st, StatusCode::OK, "set status: {v}");
    assert_eq!(v["status"], "planned");

    // Unknown status -> 422; unknown id -> 404.
    let (st, _) = s
        .post(
            &format!("/v1/admin/feedback/{fid}/status"),
            serde_json::json!({"status":"nope"}),
        )
        .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    let (st, _) = s
        .post(
            &format!("/v1/admin/feedback/{}/status", Uuid::now_v7()),
            serde_json::json!({"status":"done"}),
        )
        .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
}
