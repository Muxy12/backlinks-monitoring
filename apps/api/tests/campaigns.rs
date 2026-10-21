//! Link-building campaigns (BETA, migration 0036): the flag gates the whole
//! surface (404 when off), prospects are created manually and seeded from the
//! competitor gap, editing recomputes the score, and converting a won prospect
//! produces a protected backlink tagged with the campaign. Gated on `DATABASE_URL`.

use api::{ApiConfig, AppState};
use auth::Argon2Params;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

async fn state(beta: bool) -> Option<AppState> {
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
            campaigns_beta: beta,
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

async fn bootstrap(beta: bool) -> Option<Session> {
    let app = api::app(state(beta).await?);
    let email = format!("camp-{}@example.com", Uuid::now_v7().simple());
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
                    "name":"Acme","slug":format!("camp-{}", Uuid::now_v7().simple())
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

#[tokio::test]
async fn flag_off_hides_the_surface() {
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
    // A campaign still exists (core feature); but the BETA workflow routes 404.
    let camp = s
        .post(
            format!("/v1/organizations/{org}/campaigns"),
            serde_json::json!({"project_id": project, "name":"C"}),
        )
        .await["id"]
        .as_str()
        .unwrap()
        .to_string();
    let (st, _) = s
        .send(
            "GET",
            format!("/v1/organizations/{org}/campaigns/{camp}/prospects"),
            None,
            false,
        )
        .await;
    assert_eq!(
        st,
        StatusCode::NOT_FOUND,
        "prospects route is inert when off"
    );
    let (st, _) = s
        .send(
            "GET",
            format!("/v1/organizations/{org}/campaigns/{camp}/summary"),
            None,
            false,
        )
        .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn pipeline_seed_score_and_convert() {
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

    // A campaign extended with link-building fields.
    let camp = s
        .post(
            format!("/v1/organizations/{org}/campaigns"),
            serde_json::json!({
                "project_id": project, "name":"Q4 Links",
                "objective":"Grow topical authority",
                "primary_target_url":"https://example.com/guide",
                "start_date":"2026-01-01", "end_date":"2026-03-31"
            }),
        )
        .await;
    let camp_id = camp["id"].as_str().unwrap().to_string();
    assert_eq!(camp["objective"], "Grow topical authority");
    assert_eq!(camp["start_date"], "2026-01-01");

    // A target page.
    let (st, _) = s
        .send(
            "POST",
            format!("/v1/organizations/{org}/campaigns/{camp_id}/targets"),
            Some(serde_json::json!({"url":"https://example.com/guide","label":"Guide"})),
            true,
        )
        .await;
    assert_eq!(st, StatusCode::CREATED);

    // Build a gap: a competitor with a backlink from a domain that does not link
    // to us → that domain becomes an opportunity.
    let comp = s
        .post(
            format!("/v1/organizations/{org}/projects/{project}/competitors"),
            serde_json::json!({"domain":"rival.com"}),
        )
        .await["id"]
        .as_str()
        .unwrap()
        .to_string();
    let imp = s
        .post(
            format!("/v1/organizations/{org}/competitors/{comp}/backlinks/import"),
            serde_json::json!({"rows":[
                {"source_url":"https://authority.com/post","target_url":"https://rival.com/x","rel":"dofollow"}
            ]}),
        )
        .await;
    assert_eq!(imp["imported"], 1, "competitor backlink imported: {imp}");

    // Seed prospects from the gap.
    let seeded = s
        .post(
            format!("/v1/organizations/{org}/campaigns/{camp_id}/prospects/from-gap"),
            serde_json::json!({"min": 1}),
        )
        .await;
    assert_eq!(seeded["created"], 1, "one gap prospect seeded: {seeded}");

    // It shows up with a score and a reason, sourced from the gap.
    let (_, list) = s
        .send(
            "GET",
            format!("/v1/organizations/{org}/campaigns/{camp_id}/prospects"),
            None,
            false,
        )
        .await;
    let arr = list.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    let p = &arr[0];
    assert_eq!(p["domain"], "authority.com");
    assert_eq!(p["source"], "competitor_gap");
    assert_eq!(p["stage"], "identified");
    assert!(p["score"].as_i64().unwrap() > 0, "gap prospect scored: {p}");
    let prospect_id = p["id"].as_str().unwrap().to_string();

    // Editing to a later stage + adding a cost recomputes the score upward.
    let before = p["score"].as_i64().unwrap();
    let (st, upd) = s
        .send(
            "PATCH",
            format!("/v1/organizations/{org}/prospects/{prospect_id}"),
            Some(serde_json::json!({
                "stage":"negotiating","contact_email":"ed@authority.com",
                "estimated_cost": 150.0, "currency":"USD"
            })),
            true,
        )
        .await;
    assert_eq!(st, StatusCode::OK, "patch: {upd}");
    assert_eq!(upd["stage"], "negotiating");
    assert!(
        upd["score"].as_i64().unwrap() > before,
        "cost + contact + stage raise the score ({} -> {})",
        before,
        upd["score"]
    );

    // Convert it: a won prospect becomes a PROTECTED backlink in this campaign.
    let (st, conv) = s
        .send(
            "POST",
            format!("/v1/organizations/{org}/prospects/{prospect_id}/convert"),
            Some(serde_json::json!({
                "source_url":"https://authority.com/review",
                "cost": 150.0, "currency":"USD"
            })),
            true,
        )
        .await;
    assert_eq!(st, StatusCode::CREATED, "convert: {conv}");
    assert_eq!(conv["prospect"]["stage"], "won");
    let backlink_id = conv["backlink_id"].as_str().unwrap().to_string();
    assert_eq!(conv["prospect"]["won_backlink_id"], backlink_id);

    let (_, bl) = s
        .send(
            "GET",
            format!("/v1/organizations/{org}/backlinks/{backlink_id}"),
            None,
            false,
        )
        .await;
    assert_eq!(bl["monitoring_level"], "protected", "won link is protected");
    assert_eq!(bl["monitoring_enabled"], true);
    assert_eq!(
        bl["campaign_id"].as_str().unwrap(),
        camp_id,
        "backlink is tagged with the campaign"
    );

    // Summary reflects budget-less rollup + pipeline counts.
    let (_, sum) = s
        .send(
            "GET",
            format!("/v1/organizations/{org}/campaigns/{camp_id}/summary"),
            None,
            false,
        )
        .await;
    assert_eq!(sum["prospects_total"], 1);
    assert_eq!(
        sum["total"], 1,
        "the converted link is in the campaign rollup"
    );
}

#[tokio::test]
async fn manual_prospect_and_tenant_isolation() {
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
    let camp = s
        .post(
            format!("/v1/organizations/{org}/campaigns"),
            serde_json::json!({"project_id": project, "name":"C"}),
        )
        .await["id"]
        .as_str()
        .unwrap()
        .to_string();
    let p = s
        .post(
            format!("/v1/organizations/{org}/campaigns/{camp}/prospects"),
            serde_json::json!({"domain":"blog.example.net","contact_name":"Sam"}),
        )
        .await;
    let pid = p["id"].as_str().unwrap().to_string();
    assert_eq!(p["source"], "manual");

    // A draft is generated from the prospect + campaign context.
    let (st, draft) = s
        .send(
            "POST",
            format!("/v1/organizations/{org}/prospects/{pid}/draft"),
            Some(serde_json::json!({"sender_name":"Alex"})),
            false,
        )
        .await;
    assert_eq!(st, StatusCode::OK, "draft: {draft}");
    assert!(
        draft["subject"]
            .as_str()
            .unwrap()
            .contains("blog.example.net")
    );
    assert!(draft["body"].as_str().unwrap().contains("Hi Sam,"));

    // A second, separate workspace cannot touch the first's prospect (404).
    let other = bootstrap(true).await.unwrap();
    let other_org = other.org;
    let (st, _) = other
        .send(
            "PATCH",
            format!("/v1/organizations/{other_org}/prospects/{pid}"),
            Some(serde_json::json!({"stage":"qualified"})),
            true,
        )
        .await;
    assert_eq!(
        st,
        StatusCode::NOT_FOUND,
        "cross-tenant prospect is invisible"
    );
}
