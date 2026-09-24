//! Cross-tenant isolation over the real routes (S2, ADR-0004): org B must not
//! reach org A's project, and the response must be 404 (not 403). Gated on
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
    let cfg = ApiConfig {
        argon: Argon2Params::insecure_fast(),
        session_ttl_hours: 1,
        secure_cookies: false,
    };
    Some(AppState::new(db.pool, cfg))
}

fn jbody(v: serde_json::Value) -> Body {
    Body::from(v.to_string())
}

async fn body_json(resp: axum::response::Response) -> serde_json::Value {
    let b = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&b).unwrap_or(serde_json::Value::Null)
}

/// Register + log in a fresh user; return (session+csrf cookie header, csrf value).
async fn login_fresh(app: &axum::Router) -> (String, String) {
    let email = format!("user-{}@example.com", Uuid::now_v7().simple());
    app.clone()
        .oneshot(
            Request::post("/v1/auth/register")
                .header(header::CONTENT_TYPE, "application/json")
                .body(jbody(
                    serde_json::json!({ "email": email, "password": "correct horse", "name": "U" }),
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
                    serde_json::json!({ "email": email, "password": "correct horse" }),
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
    let pair = |name: &str| {
        cookies
            .iter()
            .find(|c| c.starts_with(name))
            .map(|c| c.split(';').next().unwrap().to_string())
            .unwrap()
    };
    let session = pair("session=");
    let csrf = pair("csrf=");
    let csrf_value = csrf.trim_start_matches("csrf=").to_string();
    (format!("{session}; {csrf}"), csrf_value)
}

async fn create_org(app: &axum::Router, cookie: &str, csrf: &str, name: &str) -> Uuid {
    let resp = app
        .clone()
        .oneshot(
            Request::post("/v1/organizations")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie)
                .header("x-csrf-token", csrf)
                .body(jbody(serde_json::json!({ "name": name })))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let v = body_json(resp).await;
    Uuid::parse_str(v["id"].as_str().unwrap()).unwrap()
}

#[tokio::test]
async fn org_b_cannot_read_org_a_project_and_gets_404() {
    let Some(state) = state().await else { return };
    let app = api::app(state);

    // Two independent tenants.
    let (a_cookie, a_csrf) = login_fresh(&app).await;
    let org_a = create_org(&app, &a_cookie, &a_csrf, "Org A").await;

    let (b_cookie, b_csrf) = login_fresh(&app).await;
    let org_b = create_org(&app, &b_cookie, &b_csrf, "Org B").await;

    // A creates a project.
    let resp = app
        .clone()
        .oneshot(
            Request::post(format!("/v1/organizations/{org_a}/projects"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &a_cookie)
                .header("x-csrf-token", &a_csrf)
                .body(jbody(
                    serde_json::json!({ "name": "A site", "domain": "a.example.com" }),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let project_id = body_json(resp).await["id"].as_str().unwrap().to_string();

    // B tries to read A's project *via B's own org path* — the project is not in
    // B's scope, so 404.
    let resp = app
        .clone()
        .oneshot(
            Request::get(format!("/v1/organizations/{org_b}/projects/{project_id}"))
                .header(header::COOKIE, &b_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "cross-tenant project read must be 404"
    );

    // B tries to read A's project *via A's org path* — B is not a member of A,
    // so authorize_org returns 404 (never 403 — IDOR, docs/05 §3).
    let resp = app
        .clone()
        .oneshot(
            Request::get(format!("/v1/organizations/{org_a}/projects/{project_id}"))
                .header(header::COOKIE, &b_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "non-member on another org must be 404"
    );

    // A can read its own project.
    let resp = app
        .oneshot(
            Request::get(format!("/v1/organizations/{org_a}/projects/{project_id}"))
                .header(header::COOKIE, &a_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
