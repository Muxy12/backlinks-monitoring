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

/// Pre-launch waitlist confirmation email — branded HTML (LinkPulz light-first
/// charte: electric blue + violet, logo) with a plain-text fallback. Via Resend,
/// best-effort (gated on the same email env; a no-op when unset).
pub mod waitlist {
    use super::resend::send_html_email;

    const LOGO: &str = "https://ye8jhaekbbcviws9.public.blob.vercel-storage.com/logo-linkpulz.webp";
    const SITE: &str = "https://linkpulz.com";

    /// Confirm a new waitlist signup. Sent once, when the email is first added.
    pub async fn send_confirmation(to: &str) -> Result<(), String> {
        let subject = "You're on the LinkPulz waitlist";
        // Plain-text fallback (kept close to the HTML copy).
        let text = "Thanks for joining the LinkPulz waitlist!\n\n\
            We're putting the finishing touches on LinkPulz — automatic backlink \
            monitoring that checks every link you earn or buy every 48 hours and \
            alerts you the moment one is lost, nofollowed, redirected or broken.\n\n\
            You'll be among the first to get access when we launch. No spam in the \
            meantime — just one email when it's ready.\n\nVisit us: https://linkpulz.com\n\n\
            — The LinkPulz team";
        let html = confirmation_html();
        send_html_email(to, subject, &html, text).await
    }

    /// Email-safe HTML (table layout, inline styles only — no external CSS, no
    /// flexbox/grid, so it renders across mail clients).
    fn confirmation_html() -> String {
        format!(
            "<!DOCTYPE html><html lang=\"en\"><head>\
<meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
<title>LinkPulz</title></head>\
<body style=\"margin:0;padding:0;background:#f6f8fc;\
font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;color:#0b1220;\">\
<table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" style=\"background:#f6f8fc;padding:28px 12px;\">\
<tr><td align=\"center\">\
<table role=\"presentation\" width=\"520\" cellpadding=\"0\" cellspacing=\"0\" \
style=\"max-width:520px;width:100%;background:#ffffff;border:1px solid #e7ecf4;border-radius:16px;overflow:hidden;\">\
<tr><td style=\"height:4px;line-height:4px;font-size:0;background:#2f6bff;\
background:linear-gradient(90deg,#2f6bff,#7c5cff);\">&nbsp;</td></tr>\
<tr><td style=\"padding:28px 32px 0;\">\
<img src=\"{LOGO}\" width=\"32\" height=\"32\" alt=\"LinkPulz\" \
style=\"vertical-align:middle;border:0;border-radius:8px;\">\
<span style=\"font-size:19px;font-weight:800;letter-spacing:-0.02em;vertical-align:middle;margin-left:8px;color:#0b1220;\">\
Link<span style=\"color:#2f6bff;\">Pulz</span></span></td></tr>\
<tr><td style=\"padding:22px 32px 0;\">\
<span style=\"display:inline-block;font-size:12px;font-weight:700;color:#2f6bff;\
background:#e8f0ff;padding:5px 12px;border-radius:999px;\">Launching soon</span>\
<h1 style=\"margin:16px 0 0;font-size:24px;line-height:1.25;font-weight:800;letter-spacing:-0.02em;color:#0b1220;\">\
You're on the waitlist</h1></td></tr>\
<tr><td style=\"padding:14px 32px 0;font-size:15px;line-height:1.65;color:#475268;\">\
Thanks for joining the LinkPulz waitlist — we're putting the finishing touches on it now.<br><br>\
LinkPulz watches every backlink you earn or buy: automatic checks \
<b style=\"color:#0b1220;\">every 48 hours</b>, with an instant alert the moment a link is lost, \
nofollowed, redirected or broken.<br><br>\
You'll be among the first to get access when we launch. No spam in the meantime — just one email when it's ready.</td></tr>\
<tr><td style=\"padding:26px 32px 4px;\">\
<table role=\"presentation\" cellpadding=\"0\" cellspacing=\"0\"><tr>\
<td style=\"border-radius:10px;background:#2f6bff;background:linear-gradient(135deg,#2f6bff,#7c5cff);\">\
<a href=\"{SITE}\" style=\"display:inline-block;padding:12px 24px;font-size:15px;font-weight:700;\
color:#ffffff;text-decoration:none;border-radius:10px;\">Visit LinkPulz &rarr;</a>\
</td></tr></table></td></tr>\
<tr><td style=\"padding:28px 32px 30px;\">\
<div style=\"border-top:1px solid #eef1f6;margin:0 0 16px;font-size:0;line-height:0;\">&nbsp;</div>\
<p style=\"margin:0;font-size:12px;line-height:1.5;color:#8a94a6;\">\
You're receiving this because you joined the LinkPulz waitlist at linkpulz.com. \
We'll only email you about the launch.</p>\
<p style=\"margin:10px 0 0;font-size:12px;color:#8a94a6;\">&copy; LinkPulz &middot; Backlink monitoring software</p>\
</td></tr></table></td></tr></table></body></html>"
        )
    }
}

pub mod resend {
    /// Email provider config (api key + from address). Err when either is unset.
    fn config() -> Result<(String, String), String> {
        let api_key = std::env::var("EMAIL_PROVIDER_API_KEY")
            .ok()
            .filter(|s| !s.is_empty());
        let from = std::env::var("EMAIL_FROM").ok().filter(|s| !s.is_empty());
        match (api_key, from) {
            (Some(k), Some(f)) => Ok((k, f)),
            _ => Err(
                "email not configured (set EMAIL_PROVIDER_API_KEY and EMAIL_FROM on the server)"
                    .into(),
            ),
        }
    }

    /// POST a prepared Resend email body. Returns Ok on 2xx, else Resend's message.
    async fn post(api_key: &str, body: &serde_json::Value) -> Result<(), String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .map_err(|e| format!("http client: {e}"))?;
        let resp = client
            .post("https://api.resend.com/emails")
            .bearer_auth(api_key)
            .json(body)
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

    /// Send a plain-text email via Resend. "email not configured" when env unset.
    pub async fn send_email(to: &str, subject: &str, text: &str) -> Result<(), String> {
        let (api_key, from) = config()?;
        let body =
            serde_json::json!({ "from": from, "to": [to], "subject": subject, "text": text });
        post(&api_key, &body).await
    }

    /// Send an HTML email (with a plain-text fallback for deliverability).
    pub async fn send_html_email(
        to: &str,
        subject: &str,
        html: &str,
        text: &str,
    ) -> Result<(), String> {
        let (api_key, from) = config()?;
        let body = serde_json::json!({
            "from": from, "to": [to], "subject": subject, "html": html, "text": text,
        });
        post(&api_key, &body).await
    }
}
