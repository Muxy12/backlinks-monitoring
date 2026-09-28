//! Multi-channel alert delivery (§37, migration 0024). Fans out new
//! notifications to each org's enabled webhook / Slack destinations, POSTing via
//! the crawler's SSRF-safe `post_json` (user-supplied URLs are guarded exactly
//! like crawled ones). Idempotent: every (destination, notification) attempt is
//! recorded in `alert_deliveries`, so a re-run never double-sends.

use crawler::CrawlerConfig;
use db::entities::AlertDestination;
use db::repositories::alerts::{self, PendingNotification};
use sqlx::PgPool;

/// Build the JSON payload for a destination kind. Slack expects `{ text }`;
/// a generic webhook gets the structured event.
fn payload(kind: &str, n: &PendingNotification) -> serde_json::Value {
    let body = n.body.clone().unwrap_or_default();
    match kind {
        "slack" => {
            let text = if body.is_empty() {
                format!("*{}*", n.title)
            } else {
                format!("*{}*\n{}", n.title, body)
            };
            serde_json::json!({ "text": text })
        }
        _ => serde_json::json!({
            "id": n.id,
            "kind": n.kind,
            "title": n.title,
            "body": n.body,
            "created_at": n.created_at.to_rfc3339(),
            "source": "backlio",
        }),
    }
}

/// Deliver one notification to one destination. Webhook/Slack go through the
/// SSRF-safe POST; email goes through Resend. Returns Ok(http_status) or an
/// error string.
async fn deliver_one(
    cfg: &CrawlerConfig,
    dest: &AlertDestination,
    n: &PendingNotification,
) -> Result<u16, String> {
    if dest.kind == "email" {
        let subject = format!("[Backlio] {}", n.title);
        let text = match &n.body {
            Some(b) if !b.is_empty() => format!("{}\n\n{}\n\n— Backlio", n.title, b),
            _ => format!("{}\n\n— Backlio", n.title),
        };
        return notifications::resend::send_email(&dest.target, &subject, &text)
            .await
            .map(|_| 200u16);
    }
    let body = payload(&dest.kind, n);
    crawler::fetch::post_json(cfg, &dest.target, &body)
        .await
        .map_err(|e| format!("{e:?}"))
}

/// Deliver pending notifications to every enabled destination. `per_dest` caps
/// how many go out per destination per tick. Returns the number sent OK.
pub async fn deliver_tick(
    pool: &PgPool,
    cfg: &CrawlerConfig,
    per_dest: i64,
) -> anyhow::Result<usize> {
    let dests = alerts::list_all_active(pool).await?;
    let mut sent = 0usize;
    for d in dests {
        let pending =
            alerts::pending_for_destination(pool, d.id, d.organization_id, per_dest).await?;
        if pending.is_empty() {
            continue;
        }
        let mut last_err: Option<String> = None;
        let mut any_ok = false;
        for n in pending {
            match deliver_one(cfg, &d, &n).await {
                Ok(code) if (200..300).contains(&code) => {
                    alerts::record_delivery(pool, d.id, n.id, "ok", Some(code as i32), None)
                        .await?;
                    sent += 1;
                    any_ok = true;
                }
                Ok(code) => {
                    let e = format!("HTTP {code}");
                    alerts::record_delivery(pool, d.id, n.id, "error", Some(code as i32), Some(&e))
                        .await?;
                    last_err = Some(e);
                }
                Err(e) => {
                    alerts::record_delivery(pool, d.id, n.id, "error", None, Some(&e)).await?;
                    last_err = Some(e);
                }
            }
        }
        let status = if last_err.is_some() && !any_ok {
            "error"
        } else {
            "ok"
        };
        let _ = alerts::set_status(pool, d.id, status, last_err.as_deref()).await;
    }
    Ok(sent)
}

/// Send a one-off test payload to a destination (the "Test" button). Records the
/// outcome on the destination but writes no delivery-ledger row. Returns the
/// HTTP status on success.
pub async fn send_test(
    pool: &PgPool,
    cfg: &CrawlerConfig,
    dest: &AlertDestination,
) -> Result<u16, String> {
    let test = PendingNotification {
        id: uuid::Uuid::now_v7(),
        kind: "test".into(),
        title: "Backlio test alert".into(),
        body: Some("If you can read this, your alert destination is wired up correctly.".into()),
        created_at: chrono::Utc::now(),
    };
    let result = deliver_one(cfg, dest, &test).await;
    match &result {
        Ok(code) if (200..300).contains(code) => {
            let _ = alerts::set_status(pool, dest.id, "ok", None).await;
        }
        Ok(code) => {
            let _ = alerts::set_status(pool, dest.id, "error", Some(&format!("HTTP {code}"))).await;
        }
        Err(e) => {
            let _ = alerts::set_status(pool, dest.id, "error", Some(e)).await;
        }
    }
    match result {
        Ok(code) if (200..300).contains(&code) => Ok(code),
        Ok(code) => Err(format!("HTTP {code}")),
        Err(e) => Err(e),
    }
}
