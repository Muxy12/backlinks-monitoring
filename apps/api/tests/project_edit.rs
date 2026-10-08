//! PATCH a project (§21): partial rename / frequency change. Gated on
//! `DATABASE_URL`.

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
    let email = format!("pe-{}@example.com", Uuid::now_v7().simple());
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
                    "name":"Acme","slug":format!("pe-{}", Uuid::now_v7().simple())
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

impl Session {
    async fn send(
        &self,
        method: &str,
        path: String,
        body: Option<serde_json::Value>,
        csrf: bool,
    ) -> (StatusCode, serde_json::Value) {
        let mut req = Request::builder().method(method).uri(path);
        req = req.header(header::COOKIE, &self.cookie);
        if csrf {
            req = req.header("x-csrf-token", &self.csrf);
        }
        let req = match body {
            Some(b) => req
                .header(header::CONTENT_TYPE, "application/json")
                .body(jbody(b))
                .unwrap(),
            None => req.body(Body::empty()).unwrap(),
        };
        jout(self.app.clone().oneshot(req).await.unwrap()).await
    }
}

#[tokio::test]
async fn patch_project_name_and_frequency() {
    let Some(s) = bootstrap().await else { return };
    let org = s.org;

    let (_, v) = s
        .send(
            "POST",
            format!("/v1/organizations/{org}/projects"),
            Some(serde_json::json!({"name":"Site","domain":"example.com"})),
            true,
        )
        .await;
    let pid = v["id"].as_str().unwrap().to_string();
    assert_eq!(v["check_frequency_secs"], 7 * 24 * 3600);
    let path = format!("/v1/organizations/{org}/projects/{pid}");

    // Rename only — frequency unchanged.
    let (st, v) = s
        .send(
            "PATCH",
            path.clone(),
            Some(serde_json::json!({"name":"Renamed"})),
            true,
        )
        .await;
    assert_eq!(st, StatusCode::OK, "patch name: {v}");
    assert_eq!(v["name"], "Renamed");
    assert_eq!(v["check_frequency_secs"], 7 * 24 * 3600, "freq kept");

    // Frequency only — name unchanged.
    let (st, v) = s
        .send(
            "PATCH",
            path.clone(),
            Some(serde_json::json!({"check_frequency_hours": 24})),
            true,
        )
        .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(v["name"], "Renamed", "name kept");
    assert_eq!(v["check_frequency_secs"], 24 * 3600);

    // Empty patch -> 422.
    let (st, _) = s
        .send("PATCH", path.clone(), Some(serde_json::json!({})), true)
        .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);

    // Out-of-range frequency -> 422.
    let (st, _) = s
        .send(
            "PATCH",
            path.clone(),
            Some(serde_json::json!({"check_frequency_hours": 99999})),
            true,
        )
        .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);

    // Missing CSRF -> 403.
    let (st, _) = s
        .send(
            "PATCH",
            path.clone(),
            Some(serde_json::json!({"name":"X"})),
            false,
        )
        .await;
    assert_eq!(st, StatusCode::FORBIDDEN);

    // Unknown project -> 404.
    let ghost = Uuid::now_v7();
    let (st, _) = s
        .send(
            "PATCH",
            format!("/v1/organizations/{org}/projects/{ghost}"),
            Some(serde_json::json!({"name":"X"})),
            true,
        )
        .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn project_client_wiring() {
    let Some(s) = bootstrap().await else { return };
    let org = s.org;

    let client = s
        .send(
            "POST",
            format!("/v1/organizations/{org}/clients"),
            Some(serde_json::json!({"name":"Client Co"})),
            true,
        )
        .await
        .1["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Create a project filed under the client.
    let (st, v) = s
        .send(
            "POST",
            format!("/v1/organizations/{org}/projects"),
            Some(serde_json::json!({"name":"Site","domain":"example.com","client_id": client})),
            true,
        )
        .await;
    assert_eq!(st, StatusCode::CREATED, "create with client: {v}");
    assert_eq!(v["client_id"], client);
    let pid = v["id"].as_str().unwrap().to_string();
    let path = format!("/v1/organizations/{org}/projects/{pid}");

    // Clear the client (explicit null).
    let (_, v) = s
        .send(
            "PATCH",
            path.clone(),
            Some(serde_json::json!({"client_id": null})),
            true,
        )
        .await;
    assert!(v["client_id"].is_null(), "client cleared");

    // Reassign the client.
    let (_, v) = s
        .send(
            "PATCH",
            path.clone(),
            Some(serde_json::json!({"client_id": client})),
            true,
        )
        .await;
    assert_eq!(v["client_id"], client, "client reassigned");

    // Creating with an unknown client -> 404.
    let ghost = Uuid::now_v7();
    let (st, _) = s
        .send(
            "POST",
            format!("/v1/organizations/{org}/projects"),
            Some(serde_json::json!({"name":"X","domain":"x.com","client_id": ghost})),
            true,
        )
        .await;
    assert_eq!(st, StatusCode::NOT_FOUND, "unknown client -> 404");
}
