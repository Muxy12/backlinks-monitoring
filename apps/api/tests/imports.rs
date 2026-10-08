//! CSV import over the real API (§38). Gated on `DATABASE_URL`.

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
async fn jout(r: axum::response::Response) -> (StatusCode, serde_json::Value) {
    let s = r.status();
    let b = r.into_body().collect().await.unwrap().to_bytes();
    (
        s,
        serde_json::from_slice(&b).unwrap_or(serde_json::Value::Null),
    )
}

#[tokio::test]
async fn csv_import_creates_backlinks_and_dedupes() {
    let Some(state) = state().await else { return };
    let app = api::app(state);
    let email = format!("i-{}@x.com", Uuid::now_v7().simple());

    app.clone()
        .oneshot(
            Request::post("/v1/auth/register")
                .header(header::CONTENT_TYPE, "application/json")
                .body(jbody(
                    serde_json::json!({"email":email,"password":"correct horse","name":"I"}),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let login = app
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
    let cookies: Vec<String> = login
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
    let csrf = pair("csrf=").trim_start_matches("csrf=").to_string();
    let cookie = format!("{}; {}", pair("session="), pair("csrf="));

    let (_, org) = jout(
        app.clone()
            .oneshot(
                Request::post("/v1/organizations")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::COOKIE, &cookie)
                    .header("x-csrf-token", &csrf)
                    .body(jbody(serde_json::json!({"name":"Acme","slug":format!("imp-{}", Uuid::now_v7().simple())})))
                    .unwrap(),
            )
            .await
            .unwrap(),
    )
    .await;
    let org_id = org["id"].as_str().expect("org created");
    let (_, proj) = jout(
        app.clone()
            .oneshot(
                Request::post(format!("/v1/organizations/{org_id}/projects"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::COOKIE, &cookie)
                    .header("x-csrf-token", &csrf)
                    .body(jbody(serde_json::json!({"name":"S","domain":"me.com"})))
                    .unwrap(),
            )
            .await
            .unwrap(),
    )
    .await;
    let project_id = proj["id"].as_str().unwrap();

    // 2 good rows + 1 bad (not a URL).
    let csv = "source_url,target_url,anchor,rel_type\n\
               https://blog.com/a,https://me.com/x,tool,dofollow\n\
               https://news.io/b,https://me.com/y,,nofollow\n\
               not-a-url,https://me.com/z,,dofollow\n";
    let import = |c: &str| {
        Request::post(format!("/v1/organizations/{org_id}/imports"))
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::COOKIE, &cookie)
            .header("x-csrf-token", &csrf)
            .body(jbody(
                serde_json::json!({ "project_id": project_id, "csv": c }),
            ))
            .unwrap()
    };

    let (st, res) = jout(app.clone().oneshot(import(csv)).await.unwrap()).await;
    assert_eq!(st, StatusCode::OK, "import: {res}");
    assert_eq!(res["imported"], 2);
    assert_eq!(
        res["errors"].as_array().unwrap().len(),
        1,
        "the non-URL row is reported"
    );

    // Re-import the same file: both good rows are duplicates now (§38).
    let (_, res2) = jout(app.clone().oneshot(import(csv)).await.unwrap()).await;
    assert_eq!(res2["imported"], 0);
    assert_eq!(res2["duplicates"], 2);

    // Import without CSRF -> 403.
    let no_csrf = Request::post(format!("/v1/organizations/{org_id}/imports"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, &cookie)
        .body(jbody(
            serde_json::json!({ "project_id": project_id, "csv": csv }),
        ))
        .unwrap();
    assert_eq!(
        app.oneshot(no_csrf).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
}
