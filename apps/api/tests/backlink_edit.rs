//! PATCH a backlink (§36): edit anchor/rel/cost/currency and assign/clear a
//! campaign & publisher, with keep-vs-clear semantics. Gated on `DATABASE_URL`.

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
}

async fn bootstrap() -> Option<Session> {
    let app = api::app(state().await?);
    let email = format!("be-{}@example.com", Uuid::now_v7().simple());
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
                    "name":"Acme","slug":format!("be-{}", Uuid::now_v7().simple())
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

#[tokio::test]
async fn patch_backlink_fields_and_assignment() {
    let Some(s) = bootstrap().await else { return };
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
    let campaign = s
        .post(
            format!("/v1/organizations/{org}/campaigns"),
            serde_json::json!({"project_id": project, "name":"Q4"}),
        )
        .await["id"]
        .as_str()
        .unwrap()
        .to_string();
    let publisher = s
        .post(
            format!("/v1/organizations/{org}/publishers"),
            serde_json::json!({"name":"Pub","domain":"pub.com"}),
        )
        .await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let bl = s
        .post(
            format!("/v1/organizations/{org}/backlinks"),
            serde_json::json!({
                "project_id": project,
                "source_url":"https://pub.com/post",
                "target_url":"https://example.com/landing",
                "expected_anchor_text":"old anchor",
                "expected_rel_type":"dofollow",
                "cost": 100.0, "currency":"USD"
            }),
        )
        .await;
    let id = bl["id"].as_str().unwrap().to_string();
    let path = format!("/v1/organizations/{org}/backlinks/{id}");

    // Edit anchor + rel; cost/currency untouched (kept).
    let (st, v) = s
        .send(
            "PATCH",
            path.clone(),
            Some(serde_json::json!({"expected_anchor_text":"new anchor","expected_rel_type":"nofollow"})),
            true,
        )
        .await;
    assert_eq!(st, StatusCode::OK, "patch: {v}");
    assert_eq!(v["expected_anchor_text"], "new anchor");
    assert_eq!(v["expected_rel_type"], "nofollow");
    assert_eq!(v["cost"], "100.00", "cost kept");
    assert_eq!(v["currency"], "USD");

    // Change cost + currency, and assign campaign + publisher.
    let (_, v) = s
        .send(
            "PATCH",
            path.clone(),
            Some(serde_json::json!({
                "cost": 250.5, "currency":"EUR",
                "campaign_id": campaign, "publisher_id": publisher
            })),
            true,
        )
        .await;
    assert_eq!(v["cost"], "250.50");
    assert_eq!(v["currency"], "EUR");
    assert_eq!(v["campaign_id"], campaign);
    assert_eq!(v["publisher_id"], publisher);

    // Clear the campaign (explicit null) — publisher stays.
    let (_, v) = s
        .send(
            "PATCH",
            path.clone(),
            Some(serde_json::json!({"campaign_id": null})),
            true,
        )
        .await;
    assert!(v["campaign_id"].is_null(), "campaign cleared");
    assert_eq!(v["publisher_id"], publisher, "publisher kept");

    // Clear cost + currency together.
    let (_, v) = s
        .send(
            "PATCH",
            path.clone(),
            Some(serde_json::json!({"cost": null, "currency": null})),
            true,
        )
        .await;
    assert!(v["cost"].is_null());
    assert!(v["currency"].is_null());

    // cost without currency -> 422.
    let (st, _) = s
        .send(
            "PATCH",
            path.clone(),
            Some(serde_json::json!({"cost": 9.0})),
            true,
        )
        .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);

    // Bad rel -> 422.
    let (st, _) = s
        .send(
            "PATCH",
            path.clone(),
            Some(serde_json::json!({"expected_rel_type":"weird"})),
            true,
        )
        .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);

    // Unknown campaign id -> 404.
    let ghost = Uuid::now_v7();
    let (st, _) = s
        .send(
            "PATCH",
            path.clone(),
            Some(serde_json::json!({"campaign_id": ghost})),
            true,
        )
        .await;
    assert_eq!(st, StatusCode::NOT_FOUND);

    // Missing CSRF -> 403.
    let (st, _) = s
        .send(
            "PATCH",
            path.clone(),
            Some(serde_json::json!({"expected_anchor_text":"x"})),
            false,
        )
        .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
}
