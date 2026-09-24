//! Hardening middleware and the public-checker rate limiter (docs/11 S3, S8).
//!
//! The checker is unauthenticated and fetches attacker-controlled URLs, so it
//! must not become a free scanner/amplifier: we cap it per client IP (a fixed
//! window) and cap total in-flight checks globally (a semaphore). This is an
//! in-process limiter — correct for a single API instance; a shared store
//! (Postgres/Redis) is the multi-instance follow-up noted in S3.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use axum::extract::Request;
use axum::http::{HeaderMap, HeaderName, HeaderValue};
use axum::middleware::Next;
use axum::response::Response;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// Why a checker request was refused.
#[derive(Debug)]
pub enum LimitError {
    /// This IP exceeded its window budget; retry after `retry_after` seconds.
    RateLimited { retry_after_secs: u64 },
    /// The global in-flight cap is full right now.
    Busy,
}

/// Per-IP fixed-window counter + a global concurrency semaphore for the public
/// backlink checker (S3).
pub struct CheckerLimiter {
    max_per_window: u32,
    window: Duration,
    buckets: Mutex<HashMap<String, (Instant, u32)>>,
    sem: std::sync::Arc<Semaphore>,
}

impl CheckerLimiter {
    /// `max_per_window` requests per `window` per IP, and at most `concurrency`
    /// checks running at once across all callers.
    pub fn new(max_per_window: u32, window: Duration, concurrency: usize) -> Self {
        CheckerLimiter {
            max_per_window,
            window,
            buckets: Mutex::new(HashMap::new()),
            sem: std::sync::Arc::new(Semaphore::new(concurrency)),
        }
    }

    /// The production default: 5 requests/hour/IP (§S3), 8 concurrent.
    pub fn default_for_prod() -> Self {
        Self::new(5, Duration::from_secs(3600), 8)
    }

    /// Admit one request from `ip`, or refuse it. On success returns a permit
    /// that must be held for the duration of the check (dropping it frees a
    /// concurrency slot). The per-IP counter is only spent once the request is
    /// admitted past the concurrency gate.
    pub fn admit(&self, ip: &str) -> Result<OwnedSemaphorePermit, LimitError> {
        // Per-IP window first (cheap, no permit held).
        {
            let now = Instant::now();
            let mut buckets = self.buckets.lock().unwrap();
            // Opportunistic prune so the map cannot grow without bound.
            if buckets.len() > 10_000 {
                buckets.retain(|_, (start, _)| now.duration_since(*start) < self.window);
            }
            let entry = buckets.entry(ip.to_string()).or_insert((now, 0));
            if now.duration_since(entry.0) >= self.window {
                *entry = (now, 0);
            }
            if entry.1 >= self.max_per_window {
                let elapsed = now.duration_since(entry.0);
                let retry = self.window.saturating_sub(elapsed);
                return Err(LimitError::RateLimited {
                    retry_after_secs: retry.as_secs().max(1),
                });
            }
            entry.1 += 1;
        }
        // Then the global concurrency cap.
        self.sem
            .clone()
            .try_acquire_owned()
            .map_err(|_| LimitError::Busy)
    }
}

/// Best-effort client IP for rate-limiting. Behind Fly the real client IP is in
/// `Fly-Client-IP`; a generic proxy uses `X-Forwarded-For` (first hop). With no
/// header (e.g. a direct local call) all callers share one bucket, which is the
/// safe default. These headers are only trusted for rate-limiting, never for
/// authz.
pub fn client_ip(headers: &HeaderMap) -> String {
    if let Some(v) = headers.get("fly-client-ip").and_then(|v| v.to_str().ok()) {
        return v.trim().to_string();
    }
    if let Some(v) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        let ip = v.split(',').next().unwrap_or("").trim();
        if !ip.is_empty() {
            return ip.to_string();
        }
    }
    "0.0.0.0".to_string()
}

/// Add conservative security headers to every response (docs/11 S8). The API
/// serves JSON, so `nosniff` + `DENY` framing + a no-referrer policy cover it;
/// the web app carries its own CSP.
pub async fn security_headers(req: Request, next: Next) -> Response {
    let mut res = next.run(req).await;
    let h = res.headers_mut();
    const PAIRS: [(&str, &str); 5] = [
        ("x-content-type-options", "nosniff"),
        ("x-frame-options", "DENY"),
        ("referrer-policy", "no-referrer"),
        ("cross-origin-opener-policy", "same-origin"),
        // HTTPS is forced at the edge (fly.toml force_https); this pins it.
        (
            "strict-transport-security",
            "max-age=31536000; includeSubDomains",
        ),
    ];
    for (k, v) in PAIRS {
        h.insert(HeaderName::from_static(k), HeaderValue::from_static(v));
    }
    res
}
