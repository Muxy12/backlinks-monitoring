//! Overview / command center aggregation (dashboard redesign). One endpoint
//! returns every section; this verifies the KPIs, the prioritized attention queue
//! (a lost PAID link is the critical, top item), campaign aggregation + top
//! opportunities (BETA), graceful behaviour when the BETA is off, tenant
//! isolation, and — the API-cost guard (§61) — that loading /overview enqueues no
//! crawler jobs. Gated on `DATABASE_URL`.

use api::{ApiConfig, AppState};
use auth::Argon2Params;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use db::sqlx::PgPool;
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

async fn build(beta: bool) -> Option<(AppState, PgPool)> {
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
            campaigns_beta: beta,
            registration_open: true,
            linkavista_api_key: None,
            linkavista_sandbox: true,
            linkavista_base_url: None,
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

async fn bootstrap(beta: bool) -> Option<Session> {
    let (state, pool) = build(beta).await?;
    let app = api::app(state);
    let email = format!("ov-{}@example.com", Uuid::now_v7().simple());
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
                    "name":"Acme","slug":format!("ov-{}", Uuid::now_v7().simple())
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
    async fn get(&self, path: String) -> serde_json::Value {
        self.send("GET", path, None, false).await.1
    }
}

#[tokio::test]
async fn overview_flag_off_is_safe_and_scoped() {
    let Some(s) = bootstrap(false).await else {
        return;
    };
    let org = s.org;
    s.post(
        format!("/v1/organizations/{org}/projects"),
        serde_json::json!({"name":"Site","domain":"example.com"}),
    )
    .await;

    let (st, v) = s
        .send(
            "GET",
            format!("/v1/organizations/{org}/overview"),
            None,
            false,
        )
        .await;
    assert_eq!(st, StatusCode::OK, "overview ok: {v}");
    // BETA off → campaign sections absent, but the Overview still works (§42).
    assert!(v["campaign_performance"].is_null());
    assert!(v["competitive_position"].is_null(), "no competitors yet");
    assert_eq!(v["top_opportunities"].as_array().unwrap().len(), 0);
    assert_eq!(v["attention"].as_array().unwrap().len(), 0, "all caught up");
    assert!(v["next_best_action"].is_null());
    assert_eq!(v["summary"]["active_campaigns"]["total"], 0);
    assert!(
        v["summary"]["link_health"]["percentage"].is_null(),
        "nothing eligible"
    );
}

#[tokio::test]
async fn overview_kpis_attention_and_campaigns() {
    let Some(s) = bootstrap(true).await else {
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

    // A paid link → monitored/protected by policy (counts toward Protected Value).
    let paid = s
        .post(
            format!("/v1/organizations/{org}/backlinks"),
            serde_json::json!({
                "project_id": project,
                "source_url":"https://techsite.com/seo-tool",
                "target_url":"https://example.com/a",
                "cost": 300.0, "currency":"USD"
            }),
        )
        .await;
    let paid_id = Uuid::parse_str(paid["id"].as_str().unwrap()).unwrap();

    // Jobs count before loading /overview — it must not enqueue any (§61).
    let jobs_before: i64 = db::sqlx::query_scalar("SELECT count(*)::bigint FROM jobs")
        .fetch_one(&s.pool)
        .await
        .unwrap();

    // Simulate the monitor having found it lost (no crawler in tests).
    db::sqlx::query("UPDATE backlinks SET status = 'lost' WHERE id = $1")
        .bind(paid_id)
        .execute(&s.pool)
        .await
        .unwrap();

    // Two snapshot days so the Link Health delta has a baseline: 50% → 90%.
    let project_uuid = Uuid::parse_str(&project).unwrap();
    db::sqlx::query(
        "INSERT INTO daily_backlink_snapshot (organization_id, project_id, day, live, monitored) \
         VALUES ($1,$2, CURRENT_DATE - 1, 5, 10), ($1,$2, CURRENT_DATE, 9, 10)",
    )
    .bind(org)
    .bind(project_uuid)
    .execute(&s.pool)
    .await
    .unwrap();

    // A campaign with a prospect awaiting a decision.
    let camp = s
        .post(
            format!("/v1/organizations/{org}/campaigns"),
            serde_json::json!({"project_id": project, "name":"Q4", "status":"active"}),
        )
        .await["id"]
        .as_str()
        .unwrap()
        .to_string();
    // create_campaign ignores status on create; set it active explicitly.
    s.send(
        "PATCH",
        format!("/v1/organizations/{org}/campaigns/{camp}"),
        Some(serde_json::json!({"status":"active"})),
        true,
    )
    .await;
    let prospect = s
        .post(
            format!("/v1/organizations/{org}/campaigns/{camp}/prospects"),
            serde_json::json!({"domain":"authority.com"}),
        )
        .await["id"]
        .as_str()
        .unwrap()
        .to_string();
    s.send(
        "PATCH",
        format!("/v1/organizations/{org}/prospects/{prospect}"),
        Some(serde_json::json!({"stage":"contacted"})),
        true,
    )
    .await;

    let v = s.get(format!("/v1/organizations/{org}/overview")).await;

    // Protected Value includes the paid link (per-currency, no FX).
    let pv = v["summary"]["protected_value"].as_array().unwrap();
    assert!(
        pv.iter()
            .any(|m| m["currency"] == "USD" && m["amount"] == "300.00"),
        "protected value: {pv:?}"
    );
    // Value at Risk picks up the now-lost paid link.
    let var = v["summary"]["value_at_risk"].as_array().unwrap();
    assert!(
        var.iter()
            .any(|m| m["currency"] == "USD" && m["amount"] == "300.00"),
        "value at risk: {var:?}"
    );
    // Attention: the lost PAID link is critical and the top (next best action).
    let att = v["attention"].as_array().unwrap();
    assert!(!att.is_empty(), "attention queue populated");
    assert_eq!(att[0]["kind"], "backlink_lost");
    assert_eq!(att[0]["priority"], "critical");
    assert_eq!(att[0]["value_at_risk"]["amount"], "300.00");
    assert_eq!(v["next_best_action"]["kind"], "backlink_lost");
    // A prospect-awaiting item is present but ranks below the critical lost link.
    assert!(
        att.iter().any(|a| a["kind"] == "prospects_awaiting"),
        "prospect awaiting item present: {att:?}"
    );

    // Active campaigns KPI + campaign performance + top opportunities.
    assert_eq!(v["summary"]["active_campaigns"]["active"], 1);
    assert!(!v["campaign_performance"].is_null());
    assert_eq!(v["campaign_performance"]["prospects_total"], 1);
    let opps = v["top_opportunities"].as_array().unwrap();
    assert_eq!(opps.len(), 1);
    assert_eq!(opps[0]["domain"], "authority.com");

    // Growth: the link counts as newly added this period.
    assert!(v["backlink_growth"]["new_count"].as_i64().unwrap() >= 1);

    // Link Health delta from the two snapshot days (50% → 90% = +40pp).
    assert_eq!(
        v["summary"]["link_health"]["change"].as_f64(),
        Some(40.0),
        "health delta: {}",
        v["summary"]["link_health"]
    );

    // §61 — loading /overview enqueued no jobs.
    let jobs_after: i64 = db::sqlx::query_scalar("SELECT count(*)::bigint FROM jobs")
        .fetch_one(&s.pool)
        .await
        .unwrap();
    assert_eq!(
        jobs_before, jobs_after,
        "/overview must not enqueue crawler jobs"
    );

    // Tenant isolation: a second workspace sees none of org A's data.
    let other = bootstrap(true).await.unwrap();
    let ov = other
        .get(format!("/v1/organizations/{}/overview", other.org))
        .await;
    assert_eq!(ov["summary"]["active_campaigns"]["total"], 0);
    assert_eq!(ov["attention"].as_array().unwrap().len(), 0);
    assert_eq!(
        ov["summary"]["protected_value"].as_array().unwrap().len(),
        0
    );
    assert!(ov["post_link"].is_null(), "no tracked links for org B");
}

// The Overview's Post-Link signal (§40): a positive tracked link surfaces, the
// read makes zero external calls (§69), and it's invisible cross-tenant.
#[tokio::test]
async fn overview_post_link_signal() {
    let Some(s) = bootstrap(false).await else {
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

    let paid_id = Uuid::parse_str(
        s.post(
            format!("/v1/organizations/{org}/backlinks"),
            serde_json::json!({
                "project_id":project,"source_url":"https://authority.com/post",
                "target_url":"https://example.com/guide","cost":200.0,"currency":"USD"
            }),
        )
        .await["id"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    let t0 = chrono::Utc::now().date_naive() - chrono::Duration::days(60);
    db::sqlx::query("UPDATE backlinks SET purchase_date = $2 WHERE id = $1")
        .bind(paid_id)
        .bind(t0)
        .execute(&s.pool)
        .await
        .unwrap();

    let (page,): (String,) =
        db::sqlx::query_as("SELECT target_url_normalized FROM backlinks WHERE id = $1")
            .bind(paid_id)
            .fetch_one(&s.pool)
            .await
            .unwrap();

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

    // Lazily create the backlink_impact row (as opening the drawer would).
    let _ = s
        .get(format!(
            "/v1/organizations/{org}/backlinks/{paid_id}/impact"
        ))
        .await;

    let jobs_before: i64 = db::sqlx::query_scalar("SELECT count(*)::bigint FROM jobs")
        .fetch_one(&s.pool)
        .await
        .unwrap();

    let ov = s.get(format!("/v1/organizations/{org}/overview")).await;
    assert!(!ov["post_link"].is_null(), "post_link present: {ov}");
    assert_eq!(ov["post_link"]["tracked"], 1);
    assert_eq!(ov["post_link"]["measured"], 1);
    assert_eq!(ov["post_link"]["positive"], 1);
    assert_eq!(ov["post_link"]["negative"], 0);

    // §69 — the Overview signal is DB-only; no crawler/GSC jobs enqueued.
    let jobs_after: i64 = db::sqlx::query_scalar("SELECT count(*)::bigint FROM jobs")
        .fetch_one(&s.pool)
        .await
        .unwrap();
    assert_eq!(
        jobs_before, jobs_after,
        "post_link signal must make no external calls"
    );

    // Cross-tenant: another workspace sees no signal from org A.
    let other = bootstrap(false).await.unwrap();
    let ov2 = other
        .get(format!("/v1/organizations/{}/overview", other.org))
        .await;
    assert!(ov2["post_link"].is_null());
}
