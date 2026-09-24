//! Public tools (§50) — no authentication. The backlink checker fetches a
//! source page and reports whether it links to a target, with the anchor and
//! rel. It is lead-gen: anyone can run one check. Because the URL is fully
//! attacker-controlled, it goes through the same SSRF-safe crawler as monitoring
//! (DNS-pinned, per-hop IP validation, redirect re-checks, body cap) — a fetch
//! that is blocked or fails returns a structured result, never a 500.
//!
//! Production note: this endpoint MUST be rate-limited per IP at the edge before
//! launch (docs/11). The crawler's short timeouts and body cap bound a single
//! request's cost, but not request volume.

use crate::error::ApiError;
use crate::security::{LimitError, client_ip};
use crate::state::AppState;
use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use backlio_core::{NormalizeOptions, Observation};
use crawler::{CrawlerConfig, FetchError, FetchOutcome, evaluate, fetch, parse_page};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CheckBody {
    pub source_url: String,
    pub target_url: String,
}

#[derive(Debug, Serialize)]
pub struct CheckResult {
    /// Whether the source page links to the target.
    pub found: bool,
    pub http_status: Option<u16>,
    /// The URL after any redirects (what was actually parsed).
    pub final_url: Option<String>,
    pub anchor_text: Option<String>,
    pub rel: Option<Vec<String>>,
    pub is_dofollow: Option<bool>,
    pub source_noindex: Option<bool>,
    /// Why the fetch could not complete (blocked/timeout/unreachable/…); null on
    /// a successful fetch regardless of whether the link was found.
    pub error: Option<String>,
}

impl CheckResult {
    fn failed(error: &str) -> Self {
        CheckResult {
            found: false,
            http_status: None,
            final_url: None,
            anchor_text: None,
            rel: None,
            is_dofollow: None,
            source_noindex: None,
            error: Some(error.to_string()),
        }
    }
}

fn is_http_url(s: &str) -> bool {
    let s = s.trim();
    s.starts_with("http://") || s.starts_with("https://")
}

fn fetch_error_kind(e: &FetchError) -> &'static str {
    match e {
        FetchError::Blocked(_) => "blocked",
        FetchError::Oversized => "response_too_large",
        FetchError::Timeout => "timeout",
        FetchError::TooManyRedirects => "too_many_redirects",
        FetchError::Connect(_) => "unreachable",
        FetchError::BadUrl => "invalid_url",
    }
}

/// `POST /v1/tools/backlink-check` — public, unauthenticated (§50). Rate limited
/// per client IP with a global concurrency cap (docs/11 S3).
pub async fn backlink_check(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CheckBody>,
) -> Result<Json<CheckResult>, ApiError> {
    // Admit past the rate limiter before doing any work; hold the permit for the
    // whole fetch so the global concurrency cap actually bounds in-flight checks.
    let _permit = match state.checker.admit(&client_ip(&headers)) {
        Ok(permit) => permit,
        Err(LimitError::RateLimited { retry_after_secs }) => {
            return Err(ApiError::RateLimited { retry_after_secs });
        }
        Err(LimitError::Busy) => return Err(ApiError::Busy),
    };

    if !is_http_url(&body.source_url) || !is_http_url(&body.target_url) {
        return Err(ApiError::Validation(
            "source_url and target_url must be http(s) URLs".into(),
        ));
    }

    let cfg = CrawlerConfig {
        allow_private_addresses: state.cfg.allow_private_fetch,
        ..CrawlerConfig::default()
    };

    let result = match fetch(&cfg, body.source_url.trim()).await {
        FetchOutcome::Response(ok) => {
            let page = parse_page(&ok.body);
            let obs = evaluate(
                &page,
                body.target_url.trim(),
                &ok.final_url,
                NormalizeOptions::default(),
            );
            match obs {
                Observation::Present(d) => CheckResult {
                    found: true,
                    http_status: Some(ok.status),
                    final_url: Some(ok.final_url),
                    anchor_text: d.anchor_text,
                    rel: Some(d.rel.to_sorted_vec()),
                    is_dofollow: Some(d.is_dofollow),
                    source_noindex: Some(d.noindex),
                    error: None,
                },
                // evaluate() only ever yields Present/Absent; treat anything
                // else as "not found on a page we did read".
                _ => CheckResult {
                    found: false,
                    http_status: Some(ok.status),
                    final_url: Some(ok.final_url),
                    anchor_text: None,
                    rel: None,
                    is_dofollow: None,
                    source_noindex: Some(page.robots_noindex),
                    error: None,
                },
            }
        }
        FetchOutcome::Failed(e) => CheckResult::failed(fetch_error_kind(&e)),
    };
    Ok(Json(result))
}
