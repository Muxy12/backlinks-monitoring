//! Selective monitoring (migration 0035): the policy picks a default stance on
//! create, the user can override it, overrides win, disabling clears the schedule
//! (so the scheduler skips the link), and bulk changes apply to a set. Gated on
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
    let email = format!("mon-{}@example.com", Uuid::now_v7().simple());
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
                    "name":"Acme","slug":format!("mon-{}", Uuid::now_v7().simple())
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
    async fn post(&self, path: String, body: serde_json::Value) -> serde_json::Value {
        self.send("POST", path, Some(body), true).await.1
    }
}

fn has_id(v: &serde_json::Value, id: &str) -> bool {
    v.as_array()
        .map(|a| a.iter().any(|b| b["id"] == id))
        .unwrap_or(false)
}

#[tokio::test]
async fn monitoring_policy_override_and_bulk() {
    let Some(s) = bootstrap().await else {
        return;
    };
    let org = s.org;
    let project = s
        .post(
            format!("/v1/organizations/{org}/projects"),
            serde_json::json!({"name":"Site","domain":"example.com"}),
        )
        .await["id"]
        .as_str()
        .unwrap()
        .to_string();

    // A plain (organic, no cost) link defaults to inventory-only: not monitored,
    // no schedule.
    let a = s
        .post(
            format!("/v1/organizations/{org}/backlinks"),
            serde_json::json!({
                "project_id": project,
                "source_url":"https://pub-a.com/post",
                "target_url":"https://example.com/a"
            }),
        )
        .await;
    let a_id = a["id"].as_str().unwrap().to_string();
    assert_eq!(a["monitoring_enabled"], false, "organic -> not monitored");
    assert_eq!(a["monitoring_level"], "inventory_only");
    assert!(
        a["next_check_at"].is_null(),
        "inventory link has no schedule"
    );

    // A paid link (cost) is monitored/protected by policy, and scheduled.
    let b = s
        .post(
            format!("/v1/organizations/{org}/backlinks"),
            serde_json::json!({
                "project_id": project,
                "source_url":"https://pub-b.com/post",
                "target_url":"https://example.com/b",
                "cost": 120.0, "currency":"USD"
            }),
        )
        .await;
    let b_id = b["id"].as_str().unwrap().to_string();
    assert_eq!(b["monitoring_enabled"], true, "paid -> monitored");
    assert_eq!(b["monitoring_level"], "protected");
    assert_eq!(b["monitoring_reason"], "economic_value");
    assert!(!b["next_check_at"].is_null(), "monitored link is scheduled");

    // User override: enable monitoring on the organic link -> scheduled, flagged.
    let path_a = format!("/v1/organizations/{org}/backlinks/{a_id}");
    let (st, v) = s
        .send(
            "PATCH",
            path_a.clone(),
            Some(serde_json::json!({"monitoring_enabled": true, "monitoring_level":"tracked"})),
            true,
        )
        .await;
    assert_eq!(st, StatusCode::OK, "enable: {v}");
    assert_eq!(v["monitoring_enabled"], true);
    assert_eq!(v["monitoring_level"], "tracked");
    assert_eq!(v["monitoring_overridden"], true, "manual change wins");
    assert!(!v["next_check_at"].is_null(), "enabling schedules it");

    // Disable it again -> schedule cleared (the scheduler will skip it).
    let (_, v) = s
        .send(
            "PATCH",
            path_a.clone(),
            Some(serde_json::json!({"monitoring_enabled": false, "monitoring_level":"inventory_only"})),
            true,
        )
        .await;
    assert_eq!(v["monitoring_enabled"], false);
    assert!(
        v["next_check_at"].is_null(),
        "disabling clears the schedule"
    );

    // Bad level -> 422.
    let (st, _) = s
        .send(
            "PATCH",
            path_a.clone(),
            Some(serde_json::json!({"monitoring_level":"super"})),
            true,
        )
        .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);

    // Tab filters.
    let list = format!("/v1/organizations/{org}/backlinks?project_id={project}");
    let (st, v) = s
        .send(
            "GET",
            format!("{list}&monitoring_level=protected"),
            None,
            false,
        )
        .await;
    assert_eq!(st, StatusCode::OK);
    assert!(
        has_id(&v, &b_id) && !has_id(&v, &a_id),
        "protected = only B"
    );
    let (_, v) = s
        .send(
            "GET",
            format!("{list}&monitoring_level=inventory_only"),
            None,
            false,
        )
        .await;
    assert!(
        has_id(&v, &a_id) && !has_id(&v, &b_id),
        "inventory = only A"
    );

    // Bulk disable B -> no longer monitored, schedule cleared.
    let (st, v) = s
        .send(
            "POST",
            format!("/v1/organizations/{org}/backlinks/bulk-monitoring"),
            Some(serde_json::json!({
                "ids": [b_id], "monitoring_enabled": false, "monitoring_level":"inventory_only"
            })),
            true,
        )
        .await;
    assert_eq!(st, StatusCode::OK, "bulk: {v}");
    assert_eq!(v["updated"], 1);
    let (_, vb) = s
        .send(
            "GET",
            format!("/v1/organizations/{org}/backlinks/{b_id}"),
            None,
            false,
        )
        .await;
    assert_eq!(vb["monitoring_enabled"], false, "bulk disabled B");
    assert!(
        vb["next_check_at"].is_null(),
        "bulk disable cleared B's schedule"
    );

    // Bulk without CSRF -> 403.
    let (st, _) = s
        .send(
            "POST",
            format!("/v1/organizations/{org}/backlinks/bulk-monitoring"),
            Some(serde_json::json!({"ids": [b_id], "monitoring_enabled": true})),
            false,
        )
        .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
}
