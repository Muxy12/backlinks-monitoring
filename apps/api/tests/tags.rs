//! Tags (§35): create, attach to a backlink, list, detach — tenant-scoped and
//! CSRF-protected. Gated on `DATABASE_URL`.

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
    let email = format!("tag-{}@example.com", Uuid::now_v7().simple());
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
                    "name":"Acme","slug":format!("tag-{}", Uuid::now_v7().simple())
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
    async fn delete(&self, path: String) -> StatusCode {
        self.app
            .clone()
            .oneshot(
                Request::delete(path)
                    .header(header::COOKIE, &self.cookie)
                    .header("x-csrf-token", &self.csrf)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap()
            .status()
    }
}

#[tokio::test]
async fn tag_create_attach_list_detach() {
    let Some(s) = bootstrap().await else { return };
    let org = s.org;

    // Project + backlink.
    let (_, v) = s
        .post(
            format!("/v1/organizations/{org}/projects"),
            serde_json::json!({"name":"Site","domain":"example.com"}),
        )
        .await;
    let project_id = v["id"].as_str().unwrap().to_string();
    let (_, v) = s
        .post(
            format!("/v1/organizations/{org}/backlinks"),
            serde_json::json!({
                "project_id": project_id,
                "source_url": "https://pub.example.com/post",
                "target_url": "https://example.com/landing",
                "expected_rel_type": "dofollow"
            }),
        )
        .await;
    let backlink_id = v["id"].as_str().unwrap().to_string();

    // Create a tag (with a color).
    let (st, v) = s
        .post(
            format!("/v1/organizations/{org}/tags"),
            serde_json::json!({"name":"outreach","color":"#3366ff"}),
        )
        .await;
    assert_eq!(st, StatusCode::CREATED, "tag: {v}");
    assert_eq!(v["name"], "outreach");
    assert_eq!(v["color"], "#3366ff");
    let tag_id = v["id"].as_str().unwrap().to_string();

    // Duplicate name (case-insensitive citext) -> 409.
    let (st, _) = s
        .post(
            format!("/v1/organizations/{org}/tags"),
            serde_json::json!({"name":"Outreach"}),
        )
        .await;
    assert_eq!(st, StatusCode::CONFLICT, "duplicate tag name conflicts");

    // Bad color -> 422.
    let (st, _) = s
        .post(
            format!("/v1/organizations/{org}/tags"),
            serde_json::json!({"name":"blue","color":"blue"}),
        )
        .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);

    // Attach the tag to the backlink -> 204, idempotent.
    for _ in 0..2 {
        let (st, _) = s
            .post(
                format!("/v1/organizations/{org}/backlinks/{backlink_id}/tags"),
                serde_json::json!({"tag_id": tag_id}),
            )
            .await;
        assert_eq!(st, StatusCode::NO_CONTENT, "attach is idempotent");
    }

    // The backlink now lists the tag.
    let (st, v) = s
        .get(format!(
            "/v1/organizations/{org}/backlinks/{backlink_id}/tags"
        ))
        .await;
    assert_eq!(st, StatusCode::OK);
    let arr = v.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["name"], "outreach");

    // Attaching an unknown tag id -> 404.
    let ghost = Uuid::now_v7();
    let (st, _) = s
        .post(
            format!("/v1/organizations/{org}/backlinks/{backlink_id}/tags"),
            serde_json::json!({"tag_id": ghost}),
        )
        .await;
    assert_eq!(st, StatusCode::NOT_FOUND, "unknown tag id -> 404");

    // Detach -> 204, and detaching again -> 404.
    assert_eq!(
        s.delete(format!(
            "/v1/organizations/{org}/backlinks/{backlink_id}/tags/{tag_id}"
        ))
        .await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        s.delete(format!(
            "/v1/organizations/{org}/backlinks/{backlink_id}/tags/{tag_id}"
        ))
        .await,
        StatusCode::NOT_FOUND
    );

    // Org tag list still has the tag; the backlink now has none.
    let (_, v) = s.get(format!("/v1/organizations/{org}/tags")).await;
    assert!(
        v.as_array()
            .unwrap()
            .iter()
            .any(|t| t["name"] == "outreach")
    );
    let (_, v) = s
        .get(format!(
            "/v1/organizations/{org}/backlinks/{backlink_id}/tags"
        ))
        .await;
    assert_eq!(v.as_array().unwrap().len(), 0);
}
