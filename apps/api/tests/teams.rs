//! Team invitations (Phase 1): an admin invites by email, the invited user
//! accepts and joins with the granted role. Tenant-scoped, CSRF-protected,
//! email-matched. Gated on `DATABASE_URL`.

use api::{ApiConfig, AppState};
use auth::Argon2Params;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

async fn app() -> Option<(axum::Router, db::sqlx::PgPool)> {
    let url = std::env::var("DATABASE_URL")
        .ok()
        .filter(|s| !s.is_empty())?;
    let db = db::build_pool(&db::DbConfig::worker(&url, 4, None))
        .await
        .ok()?;
    db::migrate::run(&db.pool).await.ok()?;
    let pool = db.pool.clone();
    let router = api::app(AppState::new(
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
    ));
    Some((router, pool))
}

/// Put an org on a paid plan with ample seats so a test can exercise the invite
/// flow rather than the Starter seat cap (1 seat = owner only, docs/20). Seat and
/// access enforcement have their own coverage.
async fn set_plan_business(pool: &db::sqlx::PgPool, org: &str) {
    db::repositories::billing::upsert_subscription(
        pool,
        Uuid::parse_str(org).unwrap(),
        "cus_test",
        None,
        "business",
        "active",
    )
    .await
    .unwrap();
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

struct Creds {
    cookie: String,
    csrf: String,
    email: String,
}

/// Register + log in a fresh user; return their cookie + csrf.
async fn signup(app: &axum::Router) -> Creds {
    let email = format!("team-{}@example.com", Uuid::now_v7().simple());
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
    let pick = |n: &str| {
        cookies
            .iter()
            .find(|c| c.starts_with(n))
            .map(|c| c.split(';').next().unwrap().to_string())
            .unwrap()
    };
    let session = pick("session=");
    let csrf_pair = pick("csrf=");
    let csrf = csrf_pair.trim_start_matches("csrf=").to_string();
    Creds {
        cookie: format!("{session}; {csrf_pair}"),
        csrf,
        email,
    }
}

async fn post(
    app: &axum::Router,
    c: &Creds,
    path: &str,
    body: serde_json::Value,
) -> (StatusCode, serde_json::Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::post(path)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &c.cookie)
                .header("x-csrf-token", &c.csrf)
                .body(jbody(body))
                .unwrap(),
        )
        .await
        .unwrap();
    jout(resp).await
}

async fn get(app: &axum::Router, c: &Creds, path: &str) -> (StatusCode, serde_json::Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::get(path)
                .header(header::COOKIE, &c.cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    jout(resp).await
}

#[tokio::test]
async fn invite_accept_and_membership() {
    let Some((app, pool)) = app().await else {
        return;
    };

    // Owner A creates an org.
    let a = signup(&app).await;
    let (_, v) = post(
        &app,
        &a,
        "/v1/organizations",
        serde_json::json!({"name":"Acme","slug":format!("team-{}", Uuid::now_v7().simple())}),
    )
    .await;
    let org = v["id"].as_str().unwrap().to_string();
    // This flow invites several people; give the workspace the seats for it.
    set_plan_business(&pool, &org).await;

    // Invitee B exists (logged in).
    let b = signup(&app).await;

    // A invites B as a member -> 201 with a one-time token.
    let (st, v) = post(
        &app,
        &a,
        &format!("/v1/organizations/{org}/invitations"),
        serde_json::json!({"email": b.email, "role": "member"}),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "invite: {v}");
    assert_eq!(v["role"], "member");
    let token = v["token"].as_str().unwrap().to_string();

    // Inviting as owner is rejected.
    let (st, _) = post(
        &app,
        &a,
        &format!("/v1/organizations/{org}/invitations"),
        serde_json::json!({"email":"x@example.com","role":"owner"}),
    )
    .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "cannot invite owner");

    // The pending invitation is listed (no token in the list).
    let (st, v) = get(&app, &a, &format!("/v1/organizations/{org}/invitations")).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(v.as_array().unwrap().len(), 1);
    assert!(v[0]["token"].is_null(), "list never leaks the token");

    // A wrong-email user cannot accept B's token.
    let intruder = signup(&app).await;
    let (st, _) = post(
        &app,
        &intruder,
        "/v1/invitations/accept",
        serde_json::json!({"token": token}),
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND, "email must match the invite");

    // B accepts -> joins as member.
    let (st, v) = post(
        &app,
        &b,
        "/v1/invitations/accept",
        serde_json::json!({"token": token}),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "accept: {v}");
    assert_eq!(v["role"], "member");
    assert_eq!(v["organization_id"], org);

    // Members list now has both, owner first.
    let (st, v) = get(&app, &a, &format!("/v1/organizations/{org}/members")).await;
    assert_eq!(st, StatusCode::OK);
    let members = v.as_array().unwrap();
    assert_eq!(members.len(), 2);
    assert_eq!(members[0]["role"], "owner");
    assert!(
        members
            .iter()
            .any(|m| m["email"] == b.email && m["role"] == "member")
    );

    // The token is spent — a second accept fails.
    let (st, _) = post(
        &app,
        &b,
        "/v1/invitations/accept",
        serde_json::json!({"token": token}),
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND, "token is single-use");

    // Revoke a fresh invitation.
    let (_, v) = post(
        &app,
        &a,
        &format!("/v1/organizations/{org}/invitations"),
        serde_json::json!({"email":"later@example.com","role":"viewer"}),
    )
    .await;
    let inv_id = v["id"].as_str().unwrap().to_string();
    let resp = app
        .clone()
        .oneshot(
            Request::delete(format!("/v1/organizations/{org}/invitations/{inv_id}"))
                .header(header::COOKIE, &a.cookie)
                .header("x-csrf-token", &a.csrf)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // B (a plain member) cannot invite anyone -> 403.
    let (st, _) = post(
        &app,
        &b,
        &format!("/v1/organizations/{org}/invitations"),
        serde_json::json!({"email":"nope@example.com","role":"member"}),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN, "members cannot invite");
}
