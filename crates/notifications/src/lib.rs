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
                format!("Your LinkPulz trial has started for {org_name}"),
                format!(
                    "Welcome to LinkPulz!\n\nYour 10-day free trial for \"{org_name}\" is active, \
                     with no credit card needed. Add your backlinks and we'll check them every 48 \
                     hours and alert you the moment one changes.\n\nChoose a plan any time:\n{billing}"
                ),
            ),
            "t_minus_3" => (
                format!("{days_left} days left in your LinkPulz trial for {org_name}"),
                format!(
                    "Your free trial for \"{org_name}\" ends in {days_left} days.\n\nTo keep \
                     monitoring your backlinks without interruption, choose a plan:\n{billing}\n\n\
                     Your data stays safe either way."
                ),
            ),
            "t_minus_1" => {
                let d = days_left.max(1);
                (
                    format!("Your LinkPulz trial ends tomorrow for {org_name}"),
                    format!(
                        "Heads-up: your free trial for \"{org_name}\" ends in {d} day{}.\n\nChoose \
                         a plan to keep monitoring without a gap:\n{billing}",
                        if d == 1 { "" } else { "s" }
                    ),
                )
            }
            _ => (
                format!("Your LinkPulz trial has ended for {org_name}"),
                format!(
                    "Your free trial for \"{org_name}\" has ended, so monitoring is paused.\n\n\
                     Your backlinks and history are safe. Choose a plan to resume:\n{billing}"
                ),
            ),
        };
        send_email(to, &subject, &body).await
    }
}

/// Pre-launch waitlist confirmation email. Deliberately sober and personal
/// (plain white, left-aligned, one small logo, a text link, no big CTA/badge/
/// gradient) so it reads as transactional and favours Gmail's Primary inbox over
/// Promotions. Includes an unsubscribe link. Via Resend, best-effort (gated on
/// the same email env; a no-op when unset).
pub mod waitlist {
    use super::resend::send_html_email;

    const LOGO: &str = "https://ye8jhaekbbcviws9.public.blob.vercel-storage.com/logo-linkpulz.webp";
    const SITE: &str = "https://linkpulz.com";

    /// The public unsubscribe URL for a signup's token.
    pub fn unsubscribe_url(token: &str) -> String {
        format!("{SITE}/waitlist/unsubscribe?t={token}")
    }

    /// Confirm a new waitlist signup. Sent once, when the email is first added.
    pub async fn send_confirmation(to: &str, unsubscribe_token: &str) -> Result<(), String> {
        let unsub = unsubscribe_url(unsubscribe_token);
        let subject = "You're on the LinkPulz waitlist";
        let text = format!(
            "Hi,\n\nThanks for joining the LinkPulz waitlist, you're all set.\n\n\
             We're putting the finishing touches on LinkPulz: automatic backlink monitoring that \
             re-checks every link you earn or buy every 48 hours and tells you the moment one is \
             lost, nofollowed, redirected or broken.\n\n\
             You'll be among the first to hear from us when early access opens. We'll only email \
             you about the launch.\n\nFollow along: {SITE}\n\nThe LinkPulz team\n\n\
             You're receiving this because you joined the LinkPulz waitlist. Unsubscribe: {unsub}"
        );
        let html = confirmation_html(&unsub);
        send_html_email(to, subject, &html, &text).await
    }

    /// Email-safe HTML (table layout, inline styles only). Intentionally plain:
    /// white background, left-aligned, no button/badge/gradient.
    fn confirmation_html(unsub: &str) -> String {
        format!(
            "<!DOCTYPE html><html lang=\"en\"><head>\
<meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
<title>LinkPulz</title></head>\
<body style=\"margin:0;padding:0;background:#ffffff;\
font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;color:#1a2234;\">\
<table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" style=\"background:#ffffff;\">\
<tr><td align=\"center\" style=\"padding:28px 16px;\">\
<table role=\"presentation\" width=\"460\" cellpadding=\"0\" cellspacing=\"0\" \
style=\"max-width:460px;width:100%;text-align:left;\">\
<tr><td style=\"padding-bottom:20px;\">\
<img src=\"{LOGO}\" width=\"26\" height=\"26\" alt=\"\" \
style=\"vertical-align:middle;border:0;border-radius:6px;\">\
<span style=\"font-size:16px;font-weight:700;vertical-align:middle;margin-left:7px;color:#1a2234;\">\
Link<span style=\"color:#2f6bff;\">Pulz</span></span></td></tr>\
<tr><td style=\"font-size:15px;line-height:1.65;color:#334155;\">\
Hi,<br><br>\
Thanks for joining the LinkPulz waitlist, you're all set.<br><br>\
We're putting the finishing touches on LinkPulz: automatic backlink monitoring that re-checks \
every link you earn or buy every 48 hours and tells you the moment one is lost, nofollowed, \
redirected or broken.<br><br>\
You'll be among the first to hear from us when early access opens. We'll only email you about \
the launch, nothing else.<br><br>\
You can follow along at <a href=\"{SITE}\" style=\"color:#2f6bff;\">linkpulz.com</a>.<br><br>\
The LinkPulz team</td></tr>\
<tr><td style=\"padding-top:28px;font-size:12px;line-height:1.5;color:#94a3b8;\">\
You're receiving this because you joined the LinkPulz waitlist. If you'd rather not hear about \
the launch, <a href=\"{unsub}\" style=\"color:#94a3b8;text-decoration:underline;\">unsubscribe here</a>.\
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

    /// Outreach config: a DEDICATED `from` (its own sending subdomain, set via
    /// `OUTREACH_FROM`) so cold-ish outreach never rides the transactional
    /// domain's reputation. Shares the `EMAIL_PROVIDER_API_KEY`. Err when unset.
    fn outreach_config() -> Result<(String, String), String> {
        let api_key = std::env::var("EMAIL_PROVIDER_API_KEY")
            .ok()
            .filter(|s| !s.is_empty());
        let from = std::env::var("OUTREACH_FROM")
            .ok()
            .filter(|s| !s.is_empty());
        match (api_key, from) {
            (Some(k), Some(f)) => Ok((k, f)),
            _ => Err(
                "outreach email not configured (set EMAIL_PROVIDER_API_KEY and OUTREACH_FROM)"
                    .into(),
            ),
        }
    }

    /// Whether outreach sending is configured (used to gate the dispatch so it
    /// stays inert until an operator sets `OUTREACH_FROM`).
    pub fn outreach_configured() -> bool {
        outreach_config().is_ok()
    }

    /// POST a prepared email and return the provider's message id on 2xx.
    async fn post_id(api_key: &str, body: &serde_json::Value) -> Result<String, String> {
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
        let detail = resp.text().await.unwrap_or_default();
        let parsed = serde_json::from_str::<serde_json::Value>(&detail).ok();
        if status.is_success() {
            let id = parsed
                .as_ref()
                .and_then(|v| v.get("id"))
                .and_then(|m| m.as_str())
                .unwrap_or_default()
                .to_string();
            Ok(id)
        } else {
            let msg = parsed
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

    /// Forward a prospect's reply to the campaign owner's inbox, from the
    /// outreach subdomain, with `reply_to` set to the prospect so the owner can
    /// answer directly. No unsubscribe header (it's a 1:1 forward, not outreach).
    pub async fn forward_reply(
        to: &str,
        subject: &str,
        text: &str,
        reply_to: &str,
    ) -> Result<(), String> {
        let (api_key, from) = outreach_config()?;
        let mut body =
            serde_json::json!({ "from": from, "to": [to], "subject": subject, "text": text });
        if !reply_to.is_empty() {
            body["reply_to"] = serde_json::json!(reply_to);
        }
        post(&api_key, &body).await
    }

    /// Send one outreach email from the dedicated subdomain. `reply_to` routes
    /// replies to the campaign owner's real inbox; the `List-Unsubscribe` header
    /// (plus the in-body link) satisfies one-click unsubscribe. Returns the
    /// provider message id. "outreach email not configured" when env is unset.
    pub async fn send_outreach(
        to: &str,
        subject: &str,
        text: &str,
        reply_to: &str,
        unsubscribe_url: &str,
    ) -> Result<String, String> {
        let (api_key, from) = outreach_config()?;
        let body = serde_json::json!({
            "from": from,
            "to": [to],
            "reply_to": reply_to,
            "subject": subject,
            "text": text,
            "headers": {
                "List-Unsubscribe": format!("<{unsubscribe_url}>"),
                "List-Unsubscribe-Post": "List-Unsubscribe=One-Click",
            },
        });
        post_id(&api_key, &body).await
    }

    /// The Resend audience (mailing list) waitlist signups are stored in.
    /// Overridable with `RESEND_AUDIENCE_ID`; defaults to the LinkPulz waitlist
    /// audience so it works without extra config once the email key is set.
    fn audience_id() -> String {
        std::env::var("RESEND_AUDIENCE_ID")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "f6725601-5a0f-4add-be5a-208944b518ac".to_string())
    }

    /// The Resend audience newsletter subscribers go into — kept SEPARATE from the
    /// waitlist so campaigns can target each list on its own. Set
    /// `RESEND_NEWSLETTER_AUDIENCE_ID`; until it is, newsletter signups fall back
    /// to the waitlist audience (no email is lost before the list is split).
    fn newsletter_audience_id() -> String {
        std::env::var("RESEND_NEWSLETTER_AUDIENCE_ID")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(audience_id)
    }

    /// A short reqwest client shared by the audience calls.
    fn http() -> Result<reqwest::Client, String> {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .map_err(|e| format!("http client: {e}"))
    }

    /// Add (upsert) a contact to the configured Resend audience. Best-effort and
    /// idempotent: a no-op when email isn't configured, and an "already exists"
    /// response counts as success so backfills and retries are safe. Marked
    /// `unsubscribed:false` since the contact just opted into the waitlist.
    pub async fn add_to_audience(email: &str) -> Result<(), String> {
        let api_key = match config() {
            Ok((k, _)) => k,
            Err(_) => return Ok(()),
        };
        add_contact(&api_key, email, &audience_id()).await
    }

    /// Add (upsert) a contact to the NEWSLETTER audience — separate from the
    /// waitlist. Best-effort: a no-op when email isn't configured.
    pub async fn add_to_newsletter(email: &str) -> Result<(), String> {
        let api_key = match config() {
            Ok((k, _)) => k,
            Err(_) => return Ok(()),
        };
        add_contact(&api_key, email, &newsletter_audience_id()).await
    }

    /// Like [`add_to_audience`] but surfaces "not configured" as an error instead
    /// of a silent no-op. Used by the admin backfill so its result reflects what
    /// actually happened (a no-op must not be reported as a successful sync).
    pub async fn add_to_audience_strict(email: &str) -> Result<(), String> {
        let (api_key, _) = config()?;
        add_contact(&api_key, email, &audience_id()).await
    }

    /// The audience id contacts are written to, so the admin UI can show which
    /// audience a sync targeted (and reveal a misconfigured `RESEND_AUDIENCE_ID`).
    pub fn audience_target() -> String {
        audience_id()
    }

    /// POST one contact to the configured audience. Ok on 2xx or an already
    /// existing contact; Err with the Resend detail otherwise.
    async fn add_contact(api_key: &str, email: &str, audience_id: &str) -> Result<(), String> {
        let url = format!("https://api.resend.com/audiences/{audience_id}/contacts");
        let body = serde_json::json!({ "email": email, "unsubscribed": false });
        let resp = http()?
            .post(&url)
            .bearer_auth(api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("resend audience request failed: {e}"))?;
        let status = resp.status();
        if status.is_success() {
            return Ok(());
        }
        let detail = resp.text().await.unwrap_or_default();
        if status == reqwest::StatusCode::CONFLICT || detail.contains("already exists") {
            return Ok(());
        }
        Err(format!(
            "resend audience HTTP {status}: {}",
            detail.chars().take(200).collect::<String>()
        ))
    }

    /// Remove a contact from the configured Resend audience by email. Best-effort;
    /// a contact that is already gone (404) counts as success.
    pub async fn remove_from_audience(email: &str) -> Result<(), String> {
        let api_key = match config() {
            Ok((k, _)) => k,
            Err(_) => return Ok(()),
        };
        // The email is already normalized at capture and its characters (@ . + - _)
        // are all valid in a URL path segment, so no extra encoding is needed.
        let url = format!(
            "https://api.resend.com/audiences/{}/contacts/{}",
            audience_id(),
            email
        );
        let resp = http()?
            .delete(&url)
            .bearer_auth(&api_key)
            .send()
            .await
            .map_err(|e| format!("resend audience delete failed: {e}"))?;
        let status = resp.status();
        if status.is_success() || status == reqwest::StatusCode::NOT_FOUND {
            return Ok(());
        }
        let detail = resp.text().await.unwrap_or_default();
        Err(format!(
            "resend audience HTTP {status}: {}",
            detail.chars().take(200).collect::<String>()
        ))
    }
}
