//! DELETE endpoints for clients / campaigns / publishers / tags, and the FK
//! side effects (backlinks unassigned, tag detached). Gated on `DATABASE_URL`.

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
    let email = format!("del-{}@example.com", Uuid::now_v7().simple());
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
                    "name":"Acme","slug":format!("del-{}", Uuid::now_v7().simple())
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
    async fn del(&self, path: String) -> StatusCode {
        self.send("DELETE", path, None, true).await.0
    }
    async fn get_json(&self, path: String) -> serde_json::Value {
        self.send("GET", path, None, false).await.1
    }
}

#[tokio::test]
async fn delete_entities_and_fk_side_effects() {
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
    let tag = s
        .post(
            format!("/v1/organizations/{org}/tags"),
            serde_json::json!({"name":"vip"}),
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
                "campaign_id": campaign,
                "publisher_id": publisher,
                "source_url":"https://pub.com/post",
                "target_url":"https://example.com/landing",
                "expected_rel_type":"dofollow"
            }),
        )
        .await;
    let bl_id = bl["id"].as_str().unwrap().to_string();
    s.post(
        format!("/v1/organizations/{org}/backlinks/{bl_id}/tags"),
        serde_json::json!({"tag_id": tag}),
    )
    .await;

    // Delete campaign -> backlink unassigned (SET NULL), publisher kept.
    assert_eq!(
        s.del(format!("/v1/organizations/{org}/campaigns/{campaign}"))
            .await,
        StatusCode::NO_CONTENT
    );
    let v = s
        .get_json(format!("/v1/organizations/{org}/backlinks/{bl_id}"))
        .await;
    assert!(v["campaign_id"].is_null(), "campaign unassigned");
    assert_eq!(v["publisher_id"], publisher, "publisher kept");

    // Delete publisher -> backlink unassigned.
    assert_eq!(
        s.del(format!("/v1/organizations/{org}/publishers/{publisher}"))
            .await,
        StatusCode::NO_CONTENT
    );
    let v = s
        .get_json(format!("/v1/organizations/{org}/backlinks/{bl_id}"))
        .await;
    assert!(v["publisher_id"].is_null(), "publisher unassigned");

    // Delete tag -> detached from the backlink.
    assert_eq!(
        s.del(format!("/v1/organizations/{org}/tags/{tag}")).await,
        StatusCode::NO_CONTENT
    );
    let v = s
        .get_json(format!("/v1/organizations/{org}/backlinks/{bl_id}/tags"))
        .await;
    assert_eq!(v.as_array().unwrap().len(), 0, "tag detached");

    // Client delete + re-delete -> 204 then 404.
    let client = s
        .post(
            format!("/v1/organizations/{org}/clients"),
            serde_json::json!({"name":"Client Co"}),
        )
        .await["id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(
        s.del(format!("/v1/organizations/{org}/clients/{client}"))
            .await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        s.del(format!("/v1/organizations/{org}/clients/{client}"))
            .await,
        StatusCode::NOT_FOUND,
        "already deleted"
    );

    // Deleting an already-deleted campaign -> 404.
    assert_eq!(
        s.del(format!("/v1/organizations/{org}/campaigns/{campaign}"))
            .await,
        StatusCode::NOT_FOUND
    );

    // Delete without CSRF -> 403.
    let tag2 = s
        .post(
            format!("/v1/organizations/{org}/tags"),
            serde_json::json!({"name":"temp"}),
        )
        .await["id"]
        .as_str()
        .unwrap()
        .to_string();
    let (st, _) = s
        .send(
            "DELETE",
            format!("/v1/organizations/{org}/tags/{tag2}"),
            None,
            false,
        )
        .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
}
