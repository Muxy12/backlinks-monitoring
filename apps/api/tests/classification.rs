//! Link classification (§ link classification, migration 0034): set the four
//! dimensions on a backlink, validate the vocabularies, and filter the list by
//! them. Gated on `DATABASE_URL`.

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
}

fn has_id(v: &serde_json::Value, id: &str) -> bool {
    v.as_array()
        .map(|a| a.iter().any(|b| b["id"] == id))
        .unwrap_or(false)
}

#[tokio::test]
async fn classify_backlink_and_filter() {
    let Some(s) = bootstrap().await else {
        return;
    };
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
    let bl = s
        .post(
            format!("/v1/organizations/{org}/backlinks"),
            serde_json::json!({
                "project_id": project,
                "source_url":"https://pub.com/post",
                "target_url":"https://example.com/landing",
                "expected_rel_type":"dofollow"
            }),
        )
        .await;
    let id = bl["id"].as_str().unwrap().to_string();
    // Unclassified at birth; never crawled -> no auto attribute.
    assert!(bl["acquisition"].is_null());
    assert!(bl["link_attribute"].is_null());
    assert!(bl["link_attribute_auto"].is_null());
    let path = format!("/v1/organizations/{org}/backlinks/{id}");

    // Set all four dimensions.
    let (st, v) = s
        .send(
            "PATCH",
            path.clone(),
            Some(serde_json::json!({
                "acquisition":"paid","placement":"footer",
                "link_format":"guest_post","link_attribute":"sponsored"
            })),
            true,
        )
        .await;
    assert_eq!(st, StatusCode::OK, "classify: {v}");
    assert_eq!(v["acquisition"], "paid");
    assert_eq!(v["placement"], "footer");
    assert_eq!(v["link_format"], "guest_post");
    assert_eq!(v["link_attribute"], "sponsored");

    // Bad value in any dimension -> 422.
    for bad in [
        serde_json::json!({"acquisition":"bribe"}),
        serde_json::json!({"placement":"roof"}),
        serde_json::json!({"link_format":"skywriting"}),
        serde_json::json!({"link_attribute":"maybe"}),
    ] {
        let (st, _) = s.send("PATCH", path.clone(), Some(bad), true).await;
        assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    }

    // Filter the list by each set dimension -> present; by a different value -> absent.
    let list = format!("/v1/organizations/{org}/backlinks?project_id={project}");
    let (st, v) = s
        .send("GET", format!("{list}&acquisition=paid"), None, false)
        .await;
    assert_eq!(st, StatusCode::OK);
    assert!(has_id(&v, &id), "acquisition=paid matches");
    let (_, v) = s
        .send("GET", format!("{list}&acquisition=earned"), None, false)
        .await;
    assert!(!has_id(&v, &id), "acquisition=earned excludes");
    // Attribute filter matches the manual override.
    let (_, v) = s
        .send("GET", format!("{list}&attribute=sponsored"), None, false)
        .await;
    assert!(has_id(&v, &id), "attribute=sponsored matches override");
    let (_, v) = s
        .send("GET", format!("{list}&placement=footer"), None, false)
        .await;
    assert!(has_id(&v, &id));
    let (_, v) = s
        .send("GET", format!("{list}&format=guest_post"), None, false)
        .await;
    assert!(has_id(&v, &id));

    // Unknown filter value -> 422.
    let (st, _) = s
        .send("GET", format!("{list}&acquisition=nope"), None, false)
        .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);

    // Clear the attribute override (null); with no crawl, the effective attribute
    // is gone, so an attribute=sponsored filter no longer matches.
    let (_, v) = s
        .send(
            "PATCH",
            path.clone(),
            Some(serde_json::json!({"link_attribute": null})),
            true,
        )
        .await;
    assert!(v["link_attribute"].is_null(), "override cleared");
    let (_, v) = s
        .send("GET", format!("{list}&attribute=sponsored"), None, false)
        .await;
    assert!(!has_id(&v, &id), "no override + no crawl -> not sponsored");

    // Clearing acquisition sets it back to unclassified.
    let (_, v) = s
        .send(
            "PATCH",
            path.clone(),
            Some(serde_json::json!({"acquisition": null})),
            true,
        )
        .await;
    assert!(v["acquisition"].is_null());
    // Other dimensions were kept (absent in the patch).
    assert_eq!(v["placement"], "footer");
    assert_eq!(v["link_format"], "guest_post");
}
