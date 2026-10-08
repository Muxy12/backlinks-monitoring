//! Google Search Console OAuth endpoints (Post-Link Impact foundation). With no
//! `GOOGLE_OAUTH_*` env the integration is inert: auth-url/sites 404, status
//! reports not-configured, the sessionless callback redirects with an error, and
//! connection mutations on a project with no connection are 404. Gated on
//! `DATABASE_URL`. (Token crypto round-trips are unit-tested in the `auth` crate.)

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
    let email = format!("gsc-{}@example.com", Uuid::now_v7().simple());
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
                    "name":"Acme","slug":format!("gsc-{}", Uuid::now_v7().simple())
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
    ) -> axum::response::Response {
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
        self.app.clone().oneshot(req).await.unwrap()
    }
}

#[tokio::test]
async fn gsc_inert_without_env() {
    // Ensure no GSC env leaks in from the test environment.
    unsafe {
        std::env::remove_var("GOOGLE_OAUTH_CLIENT_ID");
        std::env::remove_var("GOOGLE_OAUTH_CLIENT_SECRET");
        std::env::remove_var("GOOGLE_OAUTH_REDIRECT_URI");
        std::env::remove_var("GSC_TOKEN_KEY");
    }
    let Some(s) = bootstrap().await else {
        return;
    };
    let org = s.org;
    let project = {
        let r = s
            .send(
                "POST",
                format!("/v1/organizations/{org}/projects"),
                Some(serde_json::json!({"name":"Site","domain":"example.com"})),
                true,
            )
            .await;
        jout(r).await.1["id"].as_str().unwrap().to_string()
    };

    // status: reachable, reports not configured / not connected.
    let (st, v) = jout(
        s.send(
            "GET",
            format!("/v1/organizations/{org}/projects/{project}/gsc/status"),
            None,
            false,
        )
        .await,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(v["configured"], false);
    assert_eq!(v["connected"], false);

    // auth-url + sites: 404 while unconfigured.
    let r = s
        .send(
            "GET",
            format!("/v1/organizations/{org}/projects/{project}/gsc/auth-url"),
            None,
            false,
        )
        .await;
    assert_eq!(r.status(), StatusCode::NOT_FOUND);
    let r = s
        .send(
            "GET",
            format!("/v1/organizations/{org}/projects/{project}/gsc/sites"),
            None,
            false,
        )
        .await;
    assert_eq!(r.status(), StatusCode::NOT_FOUND);

    // Mutations with no connection -> 404.
    let r = s
        .send(
            "POST",
            format!("/v1/organizations/{org}/projects/{project}/gsc/site"),
            Some(serde_json::json!({"site_url":"sc-domain:example.com"})),
            true,
        )
        .await;
    assert_eq!(r.status(), StatusCode::NOT_FOUND);
    let r = s
        .send(
            "DELETE",
            format!("/v1/organizations/{org}/projects/{project}/gsc"),
            None,
            true,
        )
        .await;
    assert_eq!(r.status(), StatusCode::NOT_FOUND);

    // Public callback without env -> redirect back with an error-ish status.
    let resp = s
        .app
        .clone()
        .oneshot(
            Request::get("/v1/gsc/oauth/callback?code=x&state=y")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        resp.status().is_redirection(),
        "callback should redirect, got {}",
        resp.status()
    );
    let loc = resp
        .headers()
        .get(header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(loc.contains("gsc=unavailable"), "location: {loc}");
}
