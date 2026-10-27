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

/// Public base URL for links in emails (the web app). Falls back to prod.
fn app_url() -> String {
    std::env::var("APP_URL")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "https://linkpulz.com".to_string())
        .trim_end_matches('/')
        .to_string()
}

/// Minimal HTML escaping for values interpolated into the email body (titles and
/// bodies carry user data: domains, anchor text).
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// (accent, accent-background, badge label) for an alert, by notification kind.
/// `kind` is the event type for change alerts (`backlink_lost`, …) or a fixed
/// kind for warranty/competitor alerts.
fn alert_style(kind: &str) -> (&'static str, &'static str, &'static str) {
    match kind {
        "backlink_lost" | "source_error" | "target_error" => {
            ("#e11d48", "#fdeaef", "Backlink lost")
        }
        "backlink_restored" => ("#16a34a", "#e7f6ed", "Restored"),
        "dofollow_to_nofollow" | "noindex_detected" => ("#f59e0b", "#fef3e2", "Link devalued"),
        "anchor_changed" | "target_changed" | "source_redirected" | "target_redirected" => {
            ("#f59e0b", "#fef3e2", "Link changed")
        }
        "warranty_expiring" => ("#f59e0b", "#fef3e2", "Warranty"),
        k if k.starts_with("competitor_") => ("#2f6bff", "#e8f0ff", "Competitor"),
        _ => ("#2f6bff", "#e8f0ff", "Alert"),
    }
}

/// A branded, responsive HTML alert email (inline styles; a text wordmark avoids
/// webp/logo rendering issues across clients). Mirrors the plain-text fallback.
fn alert_email_html(n: &PendingNotification, base: &str) -> String {
    let (accent, accent_bg, badge) = alert_style(&n.kind);
    let title = esc(&n.title);
    let body_block = match &n.body {
        Some(b) if !b.is_empty() => format!(
            "<div style=\"margin:14px 0 0;padding:12px 16px;background:#f6f8fc;border:1px solid #eef2f8;\
border-radius:10px;font-size:15px;line-height:1.6;color:#334155;word-break:break-word;\">{}</div>",
            esc(b).replace('\n', "<br>")
        ),
        _ => String::new(),
    };
    format!(
        "<div style=\"background:#f4f7fb;padding:28px 12px;font-family:-apple-system,BlinkMacSystemFont,\
'Segoe UI',Roboto,Helvetica,Arial,sans-serif;\">\
<div style=\"max-width:520px;margin:0 auto;background:#ffffff;border:1px solid #e5eaf2;border-radius:14px;overflow:hidden;\">\
<div style=\"padding:20px 28px;border-bottom:1px solid #eef2f8;\">\
<img src=\"https://ye8jhaekbbcviws9.public.blob.vercel-storage.com/logo-linkpulz.webp\" alt=\"\" width=\"28\" height=\"28\" style=\"height:28px;width:28px;vertical-align:middle;margin-right:8px;border:0;\">\
<span style=\"font-size:20px;font-weight:800;letter-spacing:-.02em;color:#0f1b2d;vertical-align:middle;\">Link<span style=\"color:#2f6bff;\">Pulz</span></span>\
</div>\
<div style=\"height:4px;background:{accent};\"></div>\
<div style=\"padding:28px;\">\
<span style=\"display:inline-block;font-size:12px;font-weight:700;letter-spacing:.04em;text-transform:uppercase;\
color:{accent};background:{accent_bg};border-radius:999px;padding:4px 12px;margin-bottom:14px;\">{badge}</span>\
<h1 style=\"margin:0;font-size:20px;line-height:1.3;color:#0f1b2d;\">{title}</h1>\
{body_block}\
<a href=\"{base}/backlinks\" style=\"display:inline-block;margin-top:22px;background:#2f6bff;color:#ffffff;\
text-decoration:none;font-weight:600;font-size:15px;padding:12px 24px;border-radius:10px;\">View in LinkPulz &rarr;</a>\
</div>\
<div style=\"padding:18px 28px;border-top:1px solid #eef2f8;font-size:12px;color:#8a94a6;line-height:1.6;\">\
You're receiving this because backlink-monitoring alerts are enabled for your workspace. \
Manage them in <a href=\"{base}/settings/notifications\" style=\"color:#2f6bff;text-decoration:none;\">notification settings</a>.<br>\
LinkPulz — a service of MeedWise (SASU), Paris, France.\
</div>\
</div>\
</div>"
    )
}

/// Deliver one notification to one destination. Webhook/Slack go through the
/// SSRF-safe POST; email goes through Resend (branded HTML + text fallback).
/// Returns Ok(http_status) or an error string.
async fn deliver_one(
    cfg: &CrawlerConfig,
    dest: &AlertDestination,
    n: &PendingNotification,
) -> Result<u16, String> {
    if dest.kind == "email" {
        let subject = format!("[LinkPulz] {}", n.title);
        let text = match &n.body {
            Some(b) if !b.is_empty() => format!("{}\n\n{}\n\n— LinkPulz", n.title, b),
            _ => format!("{}\n\n— LinkPulz", n.title),
        };
        let html = alert_email_html(n, &app_url());
        return notifications::resend::send_html_email(&dest.target, &subject, &html, &text)
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
    let now = chrono::Utc::now();
    let mut sent = 0usize;
    for d in dests {
        // Digest window: batch into one message per hour/day; None = immediate.
        let window_secs = match d.digest.as_str() {
            "hourly" => Some(3600i64),
            "daily" => Some(86400i64),
            _ => None,
        };

        if let Some(win) = window_secs {
            // Only send once per window; otherwise let alerts accumulate.
            let due = d
                .last_delivered_at
                .map(|t| (now - t).num_seconds() >= win)
                .unwrap_or(true);
            if !due {
                continue;
            }
            let pending =
                alerts::pending_for_destination(pool, d.id, d.organization_id, per_dest.max(50))
                    .await?;
            if pending.is_empty() {
                continue;
            }
            sent += deliver_digest(pool, cfg, &d, &pending).await?;
        } else {
            let pending =
                alerts::pending_for_destination(pool, d.id, d.organization_id, per_dest).await?;
            if pending.is_empty() {
                continue;
            }
            sent += deliver_immediate(pool, cfg, &d, &pending).await?;
        }
    }
    Ok(sent)
}

/// One message per pending notification (digest = immediate).
async fn deliver_immediate(
    pool: &PgPool,
    cfg: &CrawlerConfig,
    d: &AlertDestination,
    pending: &[PendingNotification],
) -> anyhow::Result<usize> {
    let mut sent = 0usize;
    let mut last_err: Option<String> = None;
    let mut any_ok = false;
    for n in pending {
        match deliver_one(cfg, d, n).await {
            Ok(code) if (200..300).contains(&code) => {
                alerts::record_delivery(pool, d.id, n.id, "ok", Some(code as i32), None).await?;
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
    Ok(sent)
}

/// One combined message summarizing all pending notifications (hourly/daily).
async fn deliver_digest(
    pool: &PgPool,
    cfg: &CrawlerConfig,
    d: &AlertDestination,
    pending: &[PendingNotification],
) -> anyhow::Result<usize> {
    let n = pending.len();
    let subject = format!("{n} LinkPulz alert{}", if n > 1 { "s" } else { "" });
    let mut lines: Vec<String> = pending
        .iter()
        .take(20)
        .map(|p| format!("• {}", p.title))
        .collect();
    if n > 20 {
        lines.push(format!("…and {} more", n - 20));
    }
    let digest = PendingNotification {
        id: uuid::Uuid::now_v7(),
        kind: "digest".into(),
        title: subject,
        body: Some(lines.join("\n")),
        created_at: chrono::Utc::now(),
    };
    match deliver_one(cfg, d, &digest).await {
        Ok(code) if (200..300).contains(&code) => {
            for p in pending {
                alerts::record_delivery(pool, d.id, p.id, "ok", Some(code as i32), None).await?;
            }
            let _ = alerts::set_status(pool, d.id, "ok", None).await;
            Ok(n)
        }
        Ok(code) => {
            let e = format!("HTTP {code}");
            for p in pending {
                alerts::record_delivery(pool, d.id, p.id, "error", Some(code as i32), Some(&e))
                    .await?;
            }
            let _ = alerts::set_status(pool, d.id, "error", Some(&e)).await;
            Ok(0)
        }
        Err(e) => {
            for p in pending {
                alerts::record_delivery(pool, d.id, p.id, "error", None, Some(&e)).await?;
            }
            let _ = alerts::set_status(pool, d.id, "error", Some(&e)).await;
            Ok(0)
        }
    }
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
        title: "LinkPulz test alert".into(),
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
