//! Alert fan-out helpers (§37). Currently: the Resend email channel.
//!
//! Email is delivered through Resend's HTTP API (api.resend.com) — a fixed,
//! trusted vendor endpoint, so no SSRF guard is needed (unlike the user-supplied
//! webhook/Slack URLs, which go through the crawler's guarded POST). Gated on
//! `EMAIL_PROVIDER_API_KEY` + `EMAIL_FROM` (the names used in render.yaml and
//! docs/10); absent, `send_email` returns a clear "not configured" error that
//! surfaces on the destination's status.

/// Trial lifecycle emails (docs/20 — trial UX). Plain-text, sent via Resend, and
/// best-effort at the call site: delivery is gated on the same email env as every
/// other email, so absent that, these quietly no-op.
pub mod trial {
    use super::resend::send_email;

    fn app_url() -> String {
        std::env::var("APP_URL")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "https://web-sooty-one-68.vercel.app".to_string())
            .trim_end_matches('/')
            .to_string()
    }

    /// Send the email for `kind` (started | t_minus_3 | t_minus_1 | expired).
    /// `days_left` is the whole days remaining (only used by the reminder copy).
    pub async fn send(to: &str, org_name: &str, kind: &str, days_left: i64) -> Result<(), String> {
        let billing = format!("{}/settings/billing", app_url());
        let (subject, body) = match kind {
            "started" => (
                format!("Your LinkPulz trial has started — {org_name}"),
                format!(
                    "Welcome to LinkPulz!\n\nYour 10-day free trial for \"{org_name}\" is active \
                     — no credit card needed. Add your backlinks and we'll check them every 48 \
                     hours and alert you the moment one changes.\n\nChoose a plan any time:\n{billing}"
                ),
            ),
            "t_minus_3" => (
                format!("{days_left} days left in your LinkPulz trial — {org_name}"),
                format!(
                    "Your free trial for \"{org_name}\" ends in {days_left} days.\n\nTo keep \
                     monitoring your backlinks without interruption, choose a plan:\n{billing}\n\n\
                     Your data stays safe either way."
                ),
            ),
            "t_minus_1" => {
                let d = days_left.max(1);
                (
                    format!("Your LinkPulz trial ends tomorrow — {org_name}"),
                    format!(
                        "Heads-up: your free trial for \"{org_name}\" ends in {d} day{}.\n\nChoose \
                         a plan to keep monitoring without a gap:\n{billing}",
                        if d == 1 { "" } else { "s" }
                    ),
                )
            }
            _ => (
                format!("Your LinkPulz trial has ended — {org_name}"),
                format!(
                    "Your free trial for \"{org_name}\" has ended, so monitoring is paused.\n\n\
                     Your backlinks and history are safe. Choose a plan to resume:\n{billing}"
                ),
            ),
        };
        send_email(to, &subject, &body).await
    }
}

pub mod resend {
    /// Send a plain-text email via Resend. Returns Ok(()) on a 2xx, or a
    /// human-readable error (including "email not configured" when the env is
    /// unset, and Resend's own message on a rejection).
    pub async fn send_email(to: &str, subject: &str, text: &str) -> Result<(), String> {
        let api_key = std::env::var("EMAIL_PROVIDER_API_KEY")
            .ok()
            .filter(|s| !s.is_empty());
        let from = std::env::var("EMAIL_FROM").ok().filter(|s| !s.is_empty());
        let (api_key, from) = match (api_key, from) {
            (Some(k), Some(f)) => (k, f),
            _ => {
                return Err(
                    "email not configured (set EMAIL_PROVIDER_API_KEY and EMAIL_FROM on the server)"
                        .into(),
                );
            }
        };

        let body = serde_json::json!({
            "from": from,
            "to": [to],
            "subject": subject,
            "text": text,
        });
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .map_err(|e| format!("http client: {e}"))?;
        let resp = client
            .post("https://api.resend.com/emails")
            .bearer_auth(&api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("resend request failed: {e}"))?;
        let status = resp.status();
        if status.is_success() {
            Ok(())
        } else {
            let detail = resp.text().await.unwrap_or_default();
            let msg = serde_json::from_str::<serde_json::Value>(&detail)
                .ok()
                .and_then(|v| {
                    v.get("message")
                        .or_else(|| v.get("error"))
                        .and_then(|m| m.as_str())
                        .map(str::to_string)
                })
                .unwrap_or_else(|| detail.chars().take(200).collect());
            Err(format!("resend HTTP {status}: {msg}"))
        }
    }
}
