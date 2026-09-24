//! CSV export with formula-injection defense (§36, docs/11 S8). A crawled anchor
//! like `=cmd` must be neutralized in the exported cell. Gated on `DATABASE_URL`.

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

struct Session {
    app: axum::Router,
    cookie: String,
    csrf: String,
    org: Uuid,
}

async fn bootstrap() -> Option<Session> {
    let app = api::app(state().await?);
    let email = format!("exp-{}@example.com", Uuid::now_v7().simple());
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
                    "name":"Acme","slug":format!("exp-{}", Uuid::now_v7().simple())
                })))
                .unwrap(),
        )
        .await
        .unwrap();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let org = Uuid::parse_str(v["id"].as_str().unwrap()).unwrap();
    Some(Session {
        app,
        cookie,
        csrf,
        org,
    })
}

#[tokio::test]
async fn export_neutralizes_formula_injection() {
    let Some(s) = bootstrap().await else { return };
    let org = s.org;

    // Project.
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
    let pv: serde_json::Value =
        serde_json::from_slice(&resp.into_body().collect().await.unwrap().to_bytes()).unwrap();
    let project_id = pv["id"].as_str().unwrap().to_string();

    // Backlink whose anchor text is a spreadsheet formula (as a crawled page
    // might supply). It is accepted as text; export must neutralize it.
    let resp = s
        .app
        .clone()
        .oneshot(
            Request::post(format!("/v1/organizations/{org}/backlinks"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &s.cookie)
                .header("x-csrf-token", &s.csrf)
                .body(jbody(serde_json::json!({
                    "project_id": project_id,
                    "source_url": "https://pub.example.com/post",
                    "target_url": "https://example.com/landing",
                    "expected_anchor_text": "=SUM(1,2)",
                    "expected_rel_type": "dofollow"
                })))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Export.
    let resp = s
        .app
        .clone()
        .oneshot(
            Request::get(format!("/v1/organizations/{org}/backlinks/export"))
                .header(header::COOKIE, &s.cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let ct = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert!(ct.starts_with("text/csv"), "content-type: {ct}");
    let body = String::from_utf8(
        resp.into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .unwrap();

    // Header row present; the malicious anchor is guarded (quoted because of the
    // comma, and prefixed with ' so a spreadsheet won't execute it).
    assert!(
        body.starts_with("source_url,target_url,anchor,"),
        "header: {body}"
    );
    assert!(
        body.contains("\"'=SUM(1,2)\""),
        "anchor must be neutralized in the CSV: {body}"
    );
    // The raw, executable form must NOT appear as a bare cell value.
    assert!(
        !body.contains(",=SUM(1,2),"),
        "un-guarded formula must not appear: {body}"
    );
}
