//! Backlink list filters (§35): project, status, tag and free-text search over
//! the dynamic (QueryBuilder) list endpoint. Gated on `DATABASE_URL`.

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
    let email = format!("flt-{}@example.com", Uuid::now_v7().simple());
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
                    "name":"Acme","slug":format!("flt-{}", Uuid::now_v7().simple())
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
    async fn post(&self, path: String, body: serde_json::Value) -> serde_json::Value {
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
        jout(resp).await.1
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
    async fn len(&self, query: String) -> usize {
        let (st, v) = self.get(query).await;
        assert_eq!(st, StatusCode::OK, "list ok: {v}");
        v.as_array().unwrap().len()
    }
}

#[tokio::test]
async fn backlink_list_filters() {
    let Some(s) = bootstrap().await else { return };
    let org = s.org;

    let v = s
        .post(
            format!("/v1/organizations/{org}/projects"),
            serde_json::json!({"name":"Site","domain":"example.com"}),
        )
        .await;
    let project = v["id"].as_str().unwrap().to_string();

    // Two backlinks with different source domains.
    let mk = |src: &str| {
        serde_json::json!({
            "project_id": project,
            "source_url": src,
            "target_url": "https://example.com/landing",
            "expected_rel_type": "dofollow"
        })
    };
    let a = s
        .post(
            format!("/v1/organizations/{org}/backlinks"),
            mk("https://alpha-news.com/x"),
        )
        .await;
    let a_id = a["id"].as_str().unwrap().to_string();
    let _b = s
        .post(
            format!("/v1/organizations/{org}/backlinks"),
            mk("https://beta-blog.com/y"),
        )
        .await;

    // No filter -> both.
    assert_eq!(s.len(format!("/v1/organizations/{org}/backlinks")).await, 2);
    assert_eq!(
        s.len(format!(
            "/v1/organizations/{org}/backlinks?project_id={project}"
        ))
        .await,
        2
    );

    // Text search on the source domain -> just alpha.
    assert_eq!(
        s.len(format!("/v1/organizations/{org}/backlinks?q=alpha-news"))
            .await,
        1
    );
    assert_eq!(
        s.len(format!("/v1/organizations/{org}/backlinks?q=nothing-here"))
            .await,
        0
    );

    // Status filter: both are pending on create.
    assert_eq!(
        s.len(format!("/v1/organizations/{org}/backlinks?status=pending"))
            .await,
        2
    );
    assert_eq!(
        s.len(format!("/v1/organizations/{org}/backlinks?status=live"))
            .await,
        0
    );

    // Unknown status -> 422 (not a 500 from a bad enum cast).
    let (st, _) = s
        .get(format!("/v1/organizations/{org}/backlinks?status=bogus"))
        .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);

    // Tag filter: tag only backlink A.
    let tag = s
        .post(
            format!("/v1/organizations/{org}/tags"),
            serde_json::json!({"name":"priority"}),
        )
        .await;
    let tag_id = tag["id"].as_str().unwrap().to_string();
    let _ = s
        .post(
            format!("/v1/organizations/{org}/backlinks/{a_id}/tags"),
            serde_json::json!({"tag_id": tag_id}),
        )
        .await;
    assert_eq!(
        s.len(format!("/v1/organizations/{org}/backlinks?tag_id={tag_id}"))
            .await,
        1,
        "only the tagged backlink"
    );

    // Combined: tag + a non-matching search -> 0.
    assert_eq!(
        s.len(format!(
            "/v1/organizations/{org}/backlinks?tag_id={tag_id}&q=beta-blog"
        ))
        .await,
        0,
        "tagged link does not match the beta search"
    );

    // Client filter: a second project filed under a client, with one backlink.
    let client = s
        .post(
            format!("/v1/organizations/{org}/clients"),
            serde_json::json!({"name":"Client A"}),
        )
        .await["id"]
        .as_str()
        .unwrap()
        .to_string();
    let cproject = s
        .post(
            format!("/v1/organizations/{org}/projects"),
            serde_json::json!({"name":"Under client","domain":"c.example.com","client_id": client}),
        )
        .await["id"]
        .as_str()
        .unwrap()
        .to_string();
    s.post(
        format!("/v1/organizations/{org}/backlinks"),
        serde_json::json!({
            "project_id": cproject,
            "source_url":"https://gamma.com/z",
            "target_url":"https://example.com/3",
            "expected_rel_type":"dofollow"
        }),
    )
    .await;
    // Only the client's one backlink (the other two are in a project with no client).
    assert_eq!(
        s.len(format!(
            "/v1/organizations/{org}/backlinks?client_id={client}"
        ))
        .await,
        1,
        "client filter scopes to that client's projects"
    );
}
