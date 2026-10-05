//! Post-Link Impact (PR 3): T0 + eligibility, the visibility/ranking before-after
//! from gsc_daily, attribution collision, pending windows, no external call on
//! read, and tenant isolation. Gated on `DATABASE_URL`.

use api::{ApiConfig, AppState};
use auth::Argon2Params;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use db::sqlx::PgPool;
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

async fn build() -> Option<(AppState, PgPool)> {
    let url = std::env::var("DATABASE_URL")
        .ok()
        .filter(|s| !s.is_empty())?;
    let db = db::build_pool(&db::DbConfig::worker(&url, 4, None))
        .await
        .ok()?;
    db::migrate::run(&db.pool).await.ok()?;
    let state = AppState::new(
        db.pool.clone(),
        ApiConfig {
            argon: Argon2Params::insecure_fast(),
            session_ttl_hours: 1,
            secure_cookies: false,
            stripe_webhook_secret: None,
            allow_private_fetch: false,
            campaigns_beta: false,
        },
    );
    Some((state, db.pool))
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
    pool: PgPool,
}

async fn bootstrap() -> Option<Session> {
    let (state, pool) = build().await?;
    let app = api::app(state);
    let email = format!("imp-{}@example.com", Uuid::now_v7().simple());
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
                    "name":"Acme","slug":format!("imp-{}", Uuid::now_v7().simple())
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
        pool,
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

#[tokio::test]
async fn impact_eligibility_visibility_attribution() {
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
    let project_uuid = Uuid::parse_str(&project).unwrap();

    // Seed a GSC connection with a mapped property so visibility/ranking compute
    // from gsc_daily (no OAuth, no external call — the token bytes are a stub).
    db::sqlx::query(
        "INSERT INTO gsc_connections (id, organization_id, project_id, refresh_token_enc, site_url) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(Uuid::now_v7())
    .bind(org)
    .bind(project_uuid)
    .bind(vec![1u8, 2, 3])
    .bind("https://example.com/")
    .execute(&s.pool)
    .await
    .unwrap();

    // An organic (no cost) link is NOT eligible for impact.
    let organic = s
        .post(
            format!("/v1/organizations/{org}/backlinks"),
            serde_json::json!({"project_id":project,"source_url":"https://blog.com/x","target_url":"https://example.com/organic"}),
        )
        .await["id"]
        .as_str()
        .unwrap()
        .to_string();
    let (st, v) = s
        .send(
            "GET",
            format!("/v1/organizations/{org}/backlinks/{organic}/impact"),
            None,
            false,
        )
        .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(v["eligible"], false, "organic link not eligible");
    assert_eq!(v["status"], "unavailable");

    // A paid link IS eligible.
    let paid = s
        .post(
            format!("/v1/organizations/{org}/backlinks"),
            serde_json::json!({
                "project_id":project,"source_url":"https://authority.com/post",
                "target_url":"https://example.com/guide","cost":200.0,"currency":"USD"
            }),
        )
        .await;
    let paid_id = Uuid::parse_str(paid["id"].as_str().unwrap()).unwrap();
    // A second link to the SAME target (attribution collision).
    let paid2 = s
        .post(
            format!("/v1/organizations/{org}/backlinks"),
            serde_json::json!({
                "project_id":project,"source_url":"https://other.com/post",
                "target_url":"https://example.com/guide","cost":150.0,"currency":"USD"
            }),
        )
        .await;
    let paid2_id = Uuid::parse_str(paid2["id"].as_str().unwrap()).unwrap();

    // Pin a stable T0 60 days ago (so the 7d/28d windows are complete), for both.
    let t0 = chrono::Utc::now().date_naive() - chrono::Duration::days(60);
    for id in [paid_id, paid2_id] {
        db::sqlx::query("UPDATE backlinks SET purchase_date = $2 WHERE id = $1")
            .bind(id)
            .bind(t0)
            .execute(&s.pool)
            .await
            .unwrap();
    }

    // The target page's normalized URL (to seed gsc_daily for it).
    let (page,): (String,) =
        db::sqlx::query_as("SELECT target_url_normalized FROM backlinks WHERE id = $1")
            .bind(paid_id)
            .fetch_one(&s.pool)
            .await
            .unwrap();

    // Seed gsc_daily: 7 days before T0 (low) and 7 days from T0 (high) → a clear
    // positive visibility response, and a position improvement (8.0 → 4.0).
    let mut pages = Vec::new();
    let mut days = Vec::new();
    let mut imps = Vec::new();
    let mut clicks = Vec::new();
    let mut ctrs = Vec::new();
    let mut poss = Vec::new();
    for i in 1..=7 {
        pages.push(page.clone());
        days.push(t0 - chrono::Duration::days(i));
        imps.push(10);
        clicks.push(1);
        ctrs.push(0.1);
        poss.push(8.0);
    }
    for i in 0..7 {
        pages.push(page.clone());
        days.push(t0 + chrono::Duration::days(i));
        imps.push(30);
        clicks.push(4);
        ctrs.push(0.13);
        poss.push(4.0);
    }
    db::repositories::gsc_daily::upsert_many(
        &s.pool,
        org,
        project_uuid,
        &pages,
        &days,
        &imps,
        &clicks,
        &ctrs,
        &poss,
    )
    .await
    .unwrap();

    // Jobs before the read (the §69 cost guard).
    let jobs_before: i64 = db::sqlx::query_scalar("SELECT count(*)::bigint FROM jobs")
        .fetch_one(&s.pool)
        .await
        .unwrap();

    let (st, v) = s
        .send(
            "GET",
            format!("/v1/organizations/{org}/backlinks/{paid_id}/impact"),
            None,
            false,
        )
        .await;
    assert_eq!(st, StatusCode::OK, "impact: {v}");
    assert_eq!(v["eligible"], true);
    assert_eq!(v["t0_source"], "acquisition");
    // Crawl + Indexation are unavailable in this BETA.
    assert_eq!(v["crawl"]["status"], "unavailable");
    assert_eq!(v["indexation"]["status"], "unavailable");
    // Visibility: a 28d window with a positive change.
    let win28 = v["visibility"]["windows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["label"] == "28d")
        .cloned()
        .unwrap();
    assert_eq!(win28["status"], "positive", "visibility 28d: {win28}");
    assert!(win28["change_percent"].as_f64().unwrap() > 0.0);
    assert!(
        win28["impressions_after"].as_i64().unwrap()
            > win28["impressions_before"].as_i64().unwrap()
    );
    // Ranking: position improved (lower is better).
    assert_eq!(v["ranking"]["status"], "positive");
    assert!(
        v["ranking"]["position_after"].as_f64().unwrap()
            < v["ranking"]["position_before"].as_f64().unwrap()
    );
    // Attribution collision detected (the second link to the same page).
    assert!(
        v["context"]["overlapping_acquisitions"].as_i64().unwrap() >= 1,
        "overlap: {}",
        v["context"]
    );
    // Never claims causation.
    let summary = v["summary"].as_str().unwrap().to_lowercase();
    assert!(
        !summary.contains("caused"),
        "summary must not claim causation: {summary}"
    );

    // §69 — reading impact enqueued no jobs.
    let jobs_after: i64 = db::sqlx::query_scalar("SELECT count(*)::bigint FROM jobs")
        .fetch_one(&s.pool)
        .await
        .unwrap();
    assert_eq!(
        jobs_before, jobs_after,
        "/impact must make no external calls"
    );

    // Tenant isolation: another workspace can't read this backlink's impact.
    let other = bootstrap().await.unwrap();
    let (st, _) = other
        .send(
            "GET",
            format!("/v1/organizations/{}/backlinks/{paid_id}/impact", other.org),
            None,
            false,
        )
        .await;
    assert_eq!(
        st,
        StatusCode::NOT_FOUND,
        "cross-tenant impact is invisible"
    );
}

#[tokio::test]
async fn impact_pending_window_for_recent_link() {
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
    // A paid link acquired today → windows are pending (not fabricated).
    let paid = s
        .post(
            format!("/v1/organizations/{org}/backlinks"),
            serde_json::json!({
                "project_id":project,"source_url":"https://a.com/p",
                "target_url":"https://example.com/new","cost":100.0,"currency":"USD"
            }),
        )
        .await["id"]
        .as_str()
        .unwrap()
        .to_string();
    let (st, v) = s
        .send(
            "GET",
            format!("/v1/organizations/{org}/backlinks/{paid}/impact"),
            None,
            false,
        )
        .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(v["eligible"], true);
    // No GSC connection in this test → visibility unavailable; windows not faked.
    assert_eq!(v["visibility"]["status"], "unavailable");
    assert_eq!(v["status"], "unavailable");
}
