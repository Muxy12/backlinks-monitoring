//! OpenAI-compatible LLM client (refonte P2.3). Env-gated and provider-agnostic:
//! it talks to any `/chat/completions` endpoint, so it works with free providers
//! (Groq, Google Gemini's OpenAI endpoint, Mistral, OpenRouter) or a local
//! Ollama, with no lock-in. Configure `LLM_API_BASE`, `LLM_API_KEY`, `LLM_MODEL`.
//! When unset, the site-analysis job falls back to its heuristic baseline.

use serde::Deserialize;

pub struct LlmClient {
    base: String,
    key: String,
    model: String,
}

/// The semantic profile an LLM returns for a site. All fields optional/defaulted
/// so a partial reply still parses.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct LlmAnalysis {
    pub language: Option<String>,
    pub theme: Option<String>,
    #[serde(default)]
    pub sub_themes: Vec<String>,
    #[serde(default)]
    pub products: Vec<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
    pub summary: Option<String>,
}

impl LlmClient {
    /// Build from env, or `None` when the three vars are not all set.
    pub fn from_env() -> Option<Self> {
        let nonempty = |k: &str| std::env::var(k).ok().filter(|s| !s.is_empty());
        Some(LlmClient {
            base: nonempty("LLM_API_BASE")?.trim_end_matches('/').to_string(),
            key: nonempty("LLM_API_KEY")?,
            model: nonempty("LLM_MODEL")?,
        })
    }

    fn client() -> reqwest::Client {
        reqwest::Client::builder()
            .user_agent("LinkPulz/1.0")
            .timeout(std::time::Duration::from_secs(45))
            .build()
            .unwrap_or_default()
    }

    /// Turn a site's own text (title/meta/headings) into a semantic profile.
    /// Returns the parsed profile, or an error string (surfaced on the row).
    pub async fn analyze(&self, domain: &str, corpus: &str) -> Result<LlmAnalysis, String> {
        let corpus: String = corpus.chars().take(6000).collect();
        let sys = "You are an SEO analyst classifying a website from its own text. \
            Reply with ONLY a compact JSON object (no markdown) with keys: \
            language (BCP-47 primary code like \"fr\" or \"en\"), \
            theme (the main topic, 1-4 words), \
            sub_themes (array of 3-6 short related topics), \
            products (array of the products or services offered), \
            keywords (array of 8-15 relevant search keywords), \
            summary (one short paragraph describing the site). \
            Write the text fields in the site's own language.";
        let user = format!("Website: {domain}\n\nContent:\n{corpus}");
        let body = serde_json::json!({
            "model": self.model,
            "temperature": 0.2,
            "response_format": { "type": "json_object" },
            "messages": [
                { "role": "system", "content": sys },
                { "role": "user", "content": user },
            ],
        });
        let resp = Self::client()
            .post(format!("{}/chat/completions", self.base))
            .bearer_auth(&self.key)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("LLM request failed: {e}"))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!(
                "LLM HTTP {status}: {}",
                text.chars().take(200).collect::<String>()
            ));
        }
        let v: serde_json::Value = resp.json().await.map_err(|e| format!("LLM decode: {e}"))?;
        let content = v["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| "LLM returned no content".to_string())?;
        serde_json::from_str::<LlmAnalysis>(&strip_fences(content))
            .map_err(|e| format!("LLM JSON parse: {e}"))
    }
}

/// Some providers wrap JSON in ```json fences despite `response_format`; strip
/// them before parsing.
fn strip_fences(s: &str) -> String {
    let t = s.trim();
    let t = t
        .strip_prefix("```json")
        .or_else(|| t.strip_prefix("```"))
        .unwrap_or(t);
    let t = t.strip_suffix("```").unwrap_or(t);
    t.trim().to_string()
}
