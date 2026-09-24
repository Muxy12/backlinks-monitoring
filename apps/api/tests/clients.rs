//! Clients (§20): create with contact fields, list, delete. Gated on
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
    let email = format!("cl-{}@example.com", Uuid::now_v7().simple());
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
                    "name":"Acme","slug":format!("cl-{}", Uuid::now_v7().simple())
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
    ) -> (StatusCode, serde_json::Value) {
        let mut req = Request::builder()
            .method(method)
            .uri(path)
            .header(header::COOKIE, &self.cookie)
            .header("x-csrf-token", &self.csrf);
        if body.is_some() {
            req = req.header(header::CONTENT_TYPE, "application/json");
        }
        let req = req
            .body(body.map(jbody).unwrap_or_else(Body::empty))
            .unwrap();
        jout(self.app.clone().oneshot(req).await.unwrap()).await
    }
}

#[tokio::test]
async fn client_create_with_fields_list_delete() {
    let Some(s) = bootstrap().await else { return };
    let org = s.org;

    // Create with contact fields.
    let (st, v) = s
        .send(
            "POST",
            format!("/v1/organizations/{org}/clients"),
            Some(serde_json::json!({
                "name":"Globex","company_name":"Globex Corp","website":"https://globex.com",
                "contact_name":"Hank","contact_email":"hank@globex.com"
            })),
        )
        .await;
    assert_eq!(st, StatusCode::CREATED, "create: {v}");
    assert_eq!(v["company_name"], "Globex Corp");
    assert_eq!(v["contact_email"], "hank@globex.com");
    let id = v["id"].as_str().unwrap().to_string();

    // Bad email -> 422.
    let (st, _) = s
        .send(
            "POST",
            format!("/v1/organizations/{org}/clients"),
            Some(serde_json::json!({"name":"X","contact_email":"nope"})),
        )
        .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);

    // List shows it with the fields.
    let (_, v) = s
        .send("GET", format!("/v1/organizations/{org}/clients"), None)
        .await;
    let arr = v.as_array().unwrap();
    assert!(
        arr.iter()
            .any(|c| c["name"] == "Globex" && c["contact_name"] == "Hank")
    );

    // Delete -> 204, then gone.
    let (st, _) = s
        .send(
            "DELETE",
            format!("/v1/organizations/{org}/clients/{id}"),
            None,
        )
        .await;
    assert_eq!(st, StatusCode::NO_CONTENT);
    let (_, v) = s
        .send("GET", format!("/v1/organizations/{org}/clients"), None)
        .await;
    assert!(!v.as_array().unwrap().iter().any(|c| c["id"] == id));
}
