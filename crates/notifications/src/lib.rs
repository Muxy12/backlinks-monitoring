//! Alert fan-out helpers (§37). Currently: the Resend email channel.
//!
//! Email is delivered through Resend's HTTP API (api.resend.com) — a fixed,
//! trusted vendor endpoint, so no SSRF guard is needed (unlike the user-supplied
//! webhook/Slack URLs, which go through the crawler's guarded POST). Gated on
//! `EMAIL_PROVIDER_API_KEY` + `EMAIL_FROM` (the names used in render.yaml and
//! docs/10); absent, `send_email` returns a clear "not configured" error that
//! surfaces on the destination's status.

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
