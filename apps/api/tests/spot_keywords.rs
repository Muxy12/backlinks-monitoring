//! Spot Finder — keyword detection & review (Lot A, migration 0043). The flag
//! gates the whole surface (404 when off); detect starts a scan and enqueues the
//! `detect_site_keywords` job; manual keywords can be added/edited/kept/rejected/
//! deleted; and a project from another org is invisible (404). Gated on
//! `DATABASE_URL` like the other DB-backed tests.

use api::{ApiConfig, AppState};
use auth::Argon2Params;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use db::sqlx::PgPool;
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

async fn state(spot_finder: bool) -> Option<AppState> {
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
            spot_finder,
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
    pool: PgPool,
    cookie: String,
    csrf: String,
    org: Uuid,
}

async fn bootstrap(spot_finder: bool) -> Option<Session> {
    let st = state(spot_finder).await?;
    let pool = st.pool.clone();
    let app = api::app(st);
    let email = format!("spot-{}@example.com", Uuid::now_v7().simple());
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
                    "name":"Acme","slug":format!("spot-{}", Uuid::now_v7().simple())
                })))
                .unwrap(),
        )
        .await
        .unwrap();
    let (_, v) = jout(resp).await;
    let org = Uuid::parse_str(v["id"].as_str().unwrap()).unwrap();
    Some(Session {
        app,
        pool,
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
    async fn new_project(&self, domain: &str) -> String {
        self.send(
            "POST",
            format!("/v1/organizations/{}/projects", self.org),
            Some(serde_json::json!({"name":"Site","domain":domain})),
            true,
        )
        .await
        .1["id"]
            .as_str()
            .unwrap()
            .to_string()
    }
}

#[tokio::test]
async fn flag_off_hides_the_surface() {
    let Some(s) = bootstrap(false).await else {
        return;
    };
    let project = s.new_project("example.com").await;
    let (st, _) = s
        .send(
            "GET",
            format!("/v1/organizations/{}/projects/{project}/keywords", s.org),
            None,
            false,
        )
        .await;
    assert_eq!(
        st,
        StatusCode::NOT_FOUND,
        "keywords route is inert when off"
    );

    let (st, _) = s
        .send(
            "POST",
            format!(
                "/v1/organizations/{}/projects/{project}/keywords/detect",
                s.org
            ),
            None,
            true,
        )
        .await;
    assert_eq!(st, StatusCode::NOT_FOUND, "detect is inert when off");
}

#[tokio::test]
async fn detect_enqueues_a_scan_and_is_single_flight() {
    let Some(s) = bootstrap(true).await else {
        return;
    };
    let org = s.org;
    let project = s.new_project("spot-example.com").await;

    // Empty to start.
    let (st, v) = s
        .send(
            "GET",
            format!("/v1/organizations/{org}/projects/{project}/keywords"),
            None,
            false,
        )
        .await;
    assert_eq!(st, StatusCode::OK);
    assert!(v["scan"].is_null());
    assert_eq!(v["keywords"].as_array().unwrap().len(), 0);

    // Detect starts a queued scan and enqueues the job.
    let before = db::repositories::jobs::count_pending(&s.pool, jobs::JobKind::DetectSiteKeywords)
        .await
        .unwrap();
    let (st, scan) = s
        .send(
            "POST",
            format!("/v1/organizations/{org}/projects/{project}/keywords/detect"),
            None,
            true,
        )
        .await;
    assert_eq!(st, StatusCode::ACCEPTED);
    assert_eq!(scan["status"], "queued");
    let scan_id = scan["id"].as_str().unwrap().to_string();
    let after = db::repositories::jobs::count_pending(&s.pool, jobs::JobKind::DetectSiteKeywords)
        .await
        .unwrap();
    assert_eq!(after, before + 1, "one detection job enqueued");

    // A second detect while the first is still in flight returns the SAME scan
    // and does not enqueue another job (single scan per project).
    let (st, scan2) = s
        .send(
            "POST",
            format!("/v1/organizations/{org}/projects/{project}/keywords/detect"),
            None,
            true,
        )
        .await;
    assert_eq!(st, StatusCode::ACCEPTED);
    assert_eq!(
        scan2["id"].as_str().unwrap(),
        scan_id,
        "same in-flight scan"
    );
    let after2 = db::repositories::jobs::count_pending(&s.pool, jobs::JobKind::DetectSiteKeywords)
        .await
        .unwrap();
    assert_eq!(after2, after, "no second job while one is in flight");
}

#[tokio::test]
async fn manual_keyword_review_lifecycle() {
    let Some(s) = bootstrap(true).await else {
        return;
    };
    let org = s.org;
    let project = s.new_project("spot-review.com").await;
    let base = format!("/v1/organizations/{org}/projects/{project}/keywords");

    // Add a manual keyword — stored kept.
    let (st, kw) = s
        .send(
            "POST",
            base.clone(),
            Some(serde_json::json!({"keyword":"SEO tools"})),
            true,
        )
        .await;
    assert_eq!(st, StatusCode::CREATED);
    assert_eq!(kw["keyword"], "SEO tools");
    assert_eq!(kw["source"], "manual");
    assert_eq!(kw["status"], "kept");
    let id = kw["id"].as_str().unwrap().to_string();

    // Duplicate (case-insensitive) is refused.
    let (st, _) = s
        .send(
            "POST",
            base.clone(),
            Some(serde_json::json!({"keyword":"seo tools"})),
            true,
        )
        .await;
    assert_eq!(
        st,
        StatusCode::UNPROCESSABLE_ENTITY,
        "duplicate normalized keyword rejected"
    );

    // Reject it.
    let (st, kw) = s
        .send(
            "PATCH",
            format!("{base}/{id}"),
            Some(serde_json::json!({"status":"rejected"})),
            true,
        )
        .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(kw["status"], "rejected");

    // Rename it.
    let (st, kw) = s
        .send(
            "PATCH",
            format!("{base}/{id}"),
            Some(serde_json::json!({"keyword":"link building"})),
            true,
        )
        .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(kw["keyword"], "link building");

    // Bulk keep.
    let (st, res) = s
        .send(
            "POST",
            format!("{base}/bulk-status"),
            Some(serde_json::json!({"ids":[id],"status":"kept"})),
            true,
        )
        .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(res["updated"], 1);

    // Bad status is rejected.
    let (st, _) = s
        .send(
            "POST",
            format!("{base}/bulk-status"),
            Some(serde_json::json!({"ids":[id],"status":"nonsense"})),
            true,
        )
        .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);

    // Delete it.
    let (st, _) = s.send("DELETE", format!("{base}/{id}"), None, true).await;
    assert_eq!(st, StatusCode::NO_CONTENT);
    let (_, v) = s.send("GET", base.clone(), None, false).await;
    assert_eq!(v["keywords"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn another_orgs_project_is_invisible() {
    let Some(a) = bootstrap(true).await else {
        return;
    };
    let Some(b) = bootstrap(true).await else {
        return;
    };
    let project = a.new_project("tenant-a.com").await;
    // B uses its own org id with A's project id: the project is not in B's org,
    // so it reads as not-found (never 403) — same anti-enumeration rule as the
    // rest of the API.
    let (st, _) = b
        .send(
            "GET",
            format!("/v1/organizations/{}/projects/{project}/keywords", b.org),
            None,
            false,
        )
        .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
}

// ── Lot B: spots (SERP search) ──────────────────────────────────────────────

#[tokio::test]
async fn spots_route_is_gated_off() {
    let Some(s) = bootstrap(false).await else {
        return;
    };
    let project = s.new_project("spots-off.com").await;
    let (st, _) = s
        .send(
            "GET",
            format!("/v1/organizations/{}/projects/{project}/spots", s.org),
            None,
            false,
        )
        .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn spots_empty_and_search_needs_dataforseo() {
    let Some(s) = bootstrap(true).await else {
        return;
    };
    let org = s.org;
    let project = s.new_project("spots-on.com").await;

    // Empty to start: no run, no spots.
    let (st, v) = s
        .send(
            "GET",
            format!("/v1/organizations/{org}/projects/{project}/spots"),
            None,
            false,
        )
        .await;
    assert_eq!(st, StatusCode::OK);
    assert!(v["run"].is_null());
    assert_eq!(v["spots"].as_array().unwrap().len(), 0);

    // Search is refused when DataForSEO is not configured (the test env has no
    // credentials), before anything is spent. (A missing-credentials environment
    // is the deterministic case here; the kept-keyword guard is exercised once a
    // provider is configured.)
    let (st, _) = s
        .send(
            "POST",
            format!("/v1/organizations/{org}/projects/{project}/spots/search"),
            Some(serde_json::json!({"locale":"fr"})),
            true,
        )
        .await;
    assert_eq!(
        st,
        StatusCode::UNPROCESSABLE_ENTITY,
        "search refused without DataForSEO configured"
    );
}

#[tokio::test]
async fn enrich_is_gated_and_needs_provider() {
    // Gated off → 404.
    if let Some(s) = bootstrap(false).await {
        let project = s.new_project("enrich-off.com").await;
        let (st, _) = s
            .send(
                "POST",
                format!(
                    "/v1/organizations/{}/projects/{project}/spots/enrich",
                    s.org
                ),
                None,
                true,
            )
            .await;
        assert_eq!(st, StatusCode::NOT_FOUND);
    }
    // On, but no DataForSEO credentials in the test env → 422 (refused before
    // spending). (If creds were present, the no-spots guard also returns 422.)
    if let Some(s) = bootstrap(true).await {
        let project = s.new_project("enrich-on.com").await;
        let (st, _) = s
            .send(
                "POST",
                format!(
                    "/v1/organizations/{}/projects/{project}/spots/enrich",
                    s.org
                ),
                None,
                true,
            )
            .await;
        assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    }
}

#[tokio::test]
async fn push_to_campaign_is_gated() {
    let spot = Uuid::now_v7();
    let campaign = Uuid::now_v7();
    // Spot Finder off → 404.
    if let Some(s) = bootstrap(false).await {
        let project = s.new_project("push-off.com").await;
        let (st, _) = s
            .send(
                "POST",
                format!(
                    "/v1/organizations/{}/projects/{project}/spots/{spot}/to-campaign",
                    s.org
                ),
                Some(serde_json::json!({ "campaign_id": campaign })),
                true,
            )
            .await;
        assert_eq!(st, StatusCode::NOT_FOUND);
    }
    // Spot Finder on but campaigns BETA off → 422 (refused before touching data).
    if let Some(s) = bootstrap(true).await {
        let project = s.new_project("push-on.com").await;
        let (st, _) = s
            .send(
                "POST",
                format!(
                    "/v1/organizations/{}/projects/{project}/spots/{spot}/to-campaign",
                    s.org
                ),
                Some(serde_json::json!({ "campaign_id": campaign })),
                true,
            )
            .await;
        assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    }
}

#[tokio::test]
async fn marketplace_match_is_gated_and_needs_a_marketplace() {
    // Gated off → 404.
    if let Some(s) = bootstrap(false).await {
        let project = s.new_project("match-off.com").await;
        let (st, _) = s
            .send(
                "POST",
                format!("/v1/organizations/{}/projects/{project}/spots/match", s.org),
                None,
                true,
            )
            .await;
        assert_eq!(st, StatusCode::NOT_FOUND);
    }
    // On, but LinkAvista not configured in the test ApiConfig → 422.
    if let Some(s) = bootstrap(true).await {
        let project = s.new_project("match-on.com").await;
        let (st, _) = s
            .send(
                "POST",
                format!("/v1/organizations/{}/projects/{project}/spots/match", s.org),
                None,
                true,
            )
            .await;
        assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    }
}
