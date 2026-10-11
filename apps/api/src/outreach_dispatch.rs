//! Outreach dispatch (BETA, migration 0042). Runs on the tick: stops stale
//! sequences, then sends the due queued steps via the dedicated Resend subdomain.
//! Inert unless outreach is configured (`OUTREACH_FROM`). Every send carries a
//! reply-to (the owner's inbox), a List-Unsubscribe header and an in-body
//! unsubscribe link + physical address (CAN-SPAM / GDPR).

use backlio_core::{OutreachContext, ReclaimAngle, ReclaimPitch, outreach_render_draft};
use db::repositories::outreach;
use db::sqlx::PgPool;

/// Public base URL for links in emails (the web app, which proxies `/api/*` to
/// this API). Falls back to the prod web origin.
fn app_url() -> String {
    std::env::var("APP_URL")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "https://linkpulz.com".to_string())
        .trim_end_matches('/')
        .to_string()
}

/// The legal footer appended to every outreach email (unsubscribe + operator
/// identity). `unsub` is the one-click unsubscribe URL for this recipient.
fn footer(unsub: &str) -> String {
    format!(
        "\n\n—\nYou received this because of a link between our sites. \
Unsubscribe: {unsub}\nMeedWise (SASU), 59 rue de Ponthieu, Bureau 326, 75008 Paris, France."
    )
}

/// Send up to `limit` due steps. Returns how many were sent. Never panics; a
/// per-message failure is recorded and does not stop the batch.
pub async fn dispatch_due(pool: &PgPool, limit: i64) -> u64 {
    if !notifications::resend::outreach_configured() {
        return 0;
    }
    // Stop-on-reply / suppression guard first, so we never send a follow-up to a
    // prospect who already replied (moved out of `contacted`).
    let _ = outreach::stop_stale(pool).await;

    let due = match outreach::due_messages(pool, limit).await {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!(error = %e, "outreach: due query failed");
            return 0;
        }
    };

    let base = app_url();
    let mut sent = 0u64;
    for m in due {
        let reclaim = match (m.source.as_str(), m.origin_status.as_deref()) {
            ("reclamation", Some(status)) => {
                ReclaimAngle::from_status(status).map(|angle| ReclaimPitch {
                    angle,
                    source_url: m.origin_source_url.as_deref().unwrap_or(""),
                    anchor: m.origin_anchor.as_deref(),
                    lost_on: m.origin_lost_on.as_deref(),
                })
            }
            _ => None,
        };
        let draft = outreach_render_draft(OutreachContext {
            prospect_domain: &m.prospect_domain,
            contact_name: m.contact_name.as_deref(),
            target_url: m.target_url.as_deref(),
            sender_name: None,
            campaign_name: &m.campaign_name,
            reclaim,
        });
        let unsub = format!("{base}/api/v1/outreach/unsubscribe?m={}", m.message_id);
        let reply_to = m.owner_email.as_deref().unwrap_or("");
        let body = format!("{}{}", draft.body, footer(&unsub));

        match notifications::resend::send_outreach(
            &m.to_email,
            &draft.subject,
            &body,
            reply_to,
            &unsub,
        )
        .await
        {
            Ok(id) => {
                let _ = outreach::mark_sent(pool, m.message_id, &id).await;
                sent += 1;
            }
            Err(e) => {
                tracing::warn!(error = %e, message_id = %m.message_id, "outreach: send failed");
                let _ = outreach::mark_failed(pool, m.message_id, &e).await;
            }
        }
    }
    sent
}
