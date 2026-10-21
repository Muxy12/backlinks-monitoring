//! Notification preferences (§37): a member sets per-event/channel prefs,
//! upserts are idempotent, and bad enums are rejected. Gated on `DATABASE_URL`.

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
    let email = format!("pref-{}@example.com", Uuid::now_v7().simple());
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
                    "name":"Acme","slug":format!("pref-{}", Uuid::now_v7().simple())
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
    async fn put(&self, path: String, body: serde_json::Value) -> (StatusCode, serde_json::Value) {
        let resp = self
            .app
            .clone()
            .oneshot(
                Request::put(path)
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
}

#[tokio::test]
async fn notification_preferences_crud() {
    let Some(s) = bootstrap().await else { return };
    let org = s.org;
    let path = format!("/v1/organizations/{org}/notification-preferences");

    // Empty to start.
    let (st, v) = s.get(path.clone()).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(v.as_array().unwrap().len(), 0);

    // Mute backlink_lost email (daily digest), keep found in-app on.
    let (st, v) = s
        .put(
            path.clone(),
            serde_json::json!({"preferences":[
                {"event_type":"backlink_lost","channel":"email","enabled":false,"digest":"daily"},
                {"event_type":"backlink_found","channel":"in_app","enabled":true}
            ]}),
        )
        .await;
    assert_eq!(st, StatusCode::OK, "set prefs: {v}");
    let arr = v.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    let lost = arr
        .iter()
        .find(|p| p["event_type"] == "backlink_lost")
        .unwrap();
    assert_eq!(lost["enabled"], false);
    assert_eq!(lost["digest"], "daily");

    // Upsert the same key -> updated, not duplicated.
    let (_, v) = s
        .put(
            path.clone(),
            serde_json::json!({"preferences":[
                {"event_type":"backlink_lost","channel":"email","enabled":true}
            ]}),
        )
        .await;
    let arr = v.as_array().unwrap();
    assert_eq!(arr.len(), 2, "still two rows, not three");
    let lost = arr
        .iter()
        .find(|p| p["event_type"] == "backlink_lost")
        .unwrap();
    assert_eq!(lost["enabled"], true, "re-enabled");
    assert_eq!(lost["digest"], "immediate", "digest reset to default");

    // Bad event_type / channel / digest -> 422, and nothing is written.
    for bad in [
        serde_json::json!({"preferences":[{"event_type":"nope","channel":"email","enabled":true}]}),
        serde_json::json!({"preferences":[{"event_type":"backlink_lost","channel":"carrier_pigeon","enabled":true}]}),
        serde_json::json!({"preferences":[{"event_type":"backlink_lost","channel":"email","enabled":true,"digest":"yearly"}]}),
    ] {
        let (st, _) = s.put(path.clone(), bad).await;
        assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    }
    let (_, v) = s.get(path.clone()).await;
    assert_eq!(
        v.as_array().unwrap().len(),
        2,
        "bad writes did not add rows"
    );
}
