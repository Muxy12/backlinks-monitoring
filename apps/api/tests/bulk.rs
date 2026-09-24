//! Bulk backlink create (§35): dedup, per-row errors, batch plan-cap. Gated on
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
    let email = format!("bulk-{}@example.com", Uuid::now_v7().simple());
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
                    "name":"Acme","slug":format!("bulk-{}", Uuid::now_v7().simple())
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
}

#[tokio::test]
async fn bulk_create_dedup_and_errors() {
    let Some(s) = bootstrap().await else { return };
    let org = s.org;

    let (_, v) = s
        .post(
            format!("/v1/organizations/{org}/projects"),
            serde_json::json!({"name":"Site","domain":"example.com"}),
        )
        .await;
    let project_id = v["id"].as_str().unwrap().to_string();

    // 4 rows: 2 unique, 1 duplicate of the first, 1 invalid URL.
    let (st, v) = s
        .post(
            format!("/v1/organizations/{org}/backlinks/bulk"),
            serde_json::json!({
                "project_id": project_id,
                "backlinks": [
                    {"source_url":"https://pub-a.com/x","target_url":"https://example.com/1","expected_rel_type":"dofollow"},
                    {"source_url":"https://pub-b.com/y","target_url":"https://example.com/2"},
                    {"source_url":"https://pub-a.com/x","target_url":"https://example.com/1"},
                    {"source_url":"ftp://bad","target_url":"https://example.com/3"}
                ]
            }),
        )
        .await;
    assert_eq!(st, StatusCode::OK, "bulk: {v}");
    assert_eq!(v["total"], 4);
    assert_eq!(v["created"], 2, "two unique created");
    assert_eq!(v["duplicates"], 1, "the repeat is a duplicate");
    let errs = v["errors"].as_array().unwrap();
    assert_eq!(errs.len(), 1);
    assert_eq!(errs[0]["index"], 3, "the ftp row is the error");

    // Empty -> 422.
    let (st, _) = s
        .post(
            format!("/v1/organizations/{org}/backlinks/bulk"),
            serde_json::json!({"project_id": project_id, "backlinks": []}),
        )
        .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);

    // The two created links show up in the project list.
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
    let (_, v) = jout(resp).await;
    assert_eq!(v.as_array().unwrap().len(), 2);
}
