//! Backlink archive/restore (§36): soft-delete hides a link from lists and
//! stops monitoring; restore brings it back. Tenant-scoped, CSRF-protected.
//! Gated on `DATABASE_URL`.

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
    let email = format!("arch-{}@example.com", Uuid::now_v7().simple());
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
                    "name":"Acme","slug":format!("arch-{}", Uuid::now_v7().simple())
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
    /// POST with no body (archive/restore/check).
    async fn action(&self, path: String) -> StatusCode {
        self.app
            .clone()
            .oneshot(
                Request::post(path)
                    .header(header::COOKIE, &self.cookie)
                    .header("x-csrf-token", &self.csrf)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap()
            .status()
    }
    async fn list(&self, org: Uuid, project: &str) -> serde_json::Value {
        let resp = self
            .app
            .clone()
            .oneshot(
                Request::get(format!(
                    "/v1/organizations/{org}/backlinks?project_id={project}"
                ))
                .header(header::COOKIE, &self.cookie)
                .body(Body::empty())
                .unwrap(),
            )
            .await
            .unwrap();
        jout(resp).await.1
    }
}

#[tokio::test]
async fn archive_and_restore_a_backlink() {
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

    let (st, v) = s
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
    assert_eq!(st, StatusCode::CREATED, "backlink: {v}");
    let id = v["id"].as_str().unwrap().to_string();

    assert_eq!(s.list(org, &project_id).await.as_array().unwrap().len(), 1);

    // Archive -> 204, and it drops out of the list.
    assert_eq!(
        s.action(format!("/v1/organizations/{org}/backlinks/{id}/archive"))
            .await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        s.list(org, &project_id).await.as_array().unwrap().len(),
        0,
        "archived link is hidden"
    );

    // Archiving again -> 404 (already archived).
    assert_eq!(
        s.action(format!("/v1/organizations/{org}/backlinks/{id}/archive"))
            .await,
        StatusCode::NOT_FOUND
    );

    // Restore -> 204, and it reappears.
    assert_eq!(
        s.action(format!("/v1/organizations/{org}/backlinks/{id}/restore"))
            .await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        s.list(org, &project_id).await.as_array().unwrap().len(),
        1,
        "restored link is back"
    );

    // Restoring a live link -> 404 (not archived).
    assert_eq!(
        s.action(format!("/v1/organizations/{org}/backlinks/{id}/restore"))
            .await,
        StatusCode::NOT_FOUND
    );

    // Unknown id -> 404.
    let ghost = Uuid::now_v7();
    assert_eq!(
        s.action(format!("/v1/organizations/{org}/backlinks/{ghost}/archive"))
            .await,
        StatusCode::NOT_FOUND
    );

    // Archive without CSRF -> 403.
    let resp = s
        .app
        .clone()
        .oneshot(
            Request::post(format!("/v1/organizations/{org}/backlinks/{id}/archive"))
                .header(header::COOKIE, &s.cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}
