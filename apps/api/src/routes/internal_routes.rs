//! Internal, machine-triggered endpoints (docs/17). On the free/cron hosting
//! model there is no always-on worker: a scheduled job calls `POST
//! /internal/tick` every few minutes, and the API does one round of monitoring
//! in-process — schedule due checks, process a small batch (the crawl), and run
//! the warranty sweep. Authenticated by the `SCHEDULER_SECRET` shared secret,
//! not a user session. Backlink checks are inherently low-frequency (default
//! every 7 days), so a periodic tick is more than enough.

use crate::error::ApiError;
use crate::state::AppState;
use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use backlio_core::TransitionPolicy;
use common::{AppEnv, Guards};
use crawler::CrawlerConfig;
use jobs::BackoffPolicy;
use std::str::FromStr;
use std::time::Duration;

/// Constant-time byte comparison (no early return on length-equal inputs).
fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b) {
        diff |= x ^ y;
    }
    diff == 0
}

pub async fn tick(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    // The secret is read from the env; absent -> the endpoint is disabled (404).
    let secret = std::env::var("SCHEDULER_SECRET")
        .ok()
        .filter(|s| !s.is_empty())
        .ok_or(ApiError::NotFound)?;
    let provided = headers
        .get("x-scheduler-secret")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !ct_eq(provided.as_bytes(), secret.as_bytes()) {
        return Err(ApiError::Unauthorized);
    }

    // Crawling/processing is gated by the environment (docs/10 §1): production
    // always, development only with WORKER_ENABLE=true, preview never.
    let app_env = std::env::var("APP_ENV")
        .ok()
        .and_then(|s| AppEnv::from_str(&s).ok())
        .unwrap_or(AppEnv::Development);
    let dev_opt_in = std::env::var("WORKER_ENABLE").as_deref() == Ok("true");
    let guards = Guards::new(app_env, dev_opt_in);

    let ctx = worker::WorkerCtx {
        pool: state.pool.clone(),
        // Production monitoring resolves the target's final URL too (SSRF-safe).
        // Tight per-fetch timeouts: the whole tick is ONE HTTP request on the
        // free tier, so a handful of slow/dead sources must not dominate it.
        crawler: CrawlerConfig {
            check_target: true,
            request_timeout: Duration::from_secs(8),
            total_timeout: Duration::from_secs(10),
            ..CrawlerConfig::default()
        },
        policy: TransitionPolicy::default(),
        backoff: BackoffPolicy::default(),
        worker_id: "api-tick".to_string(),
    };

    // Enqueue due checks (safe in any env — advisory-locked, no crawling, fast).
    let (scheduled_jobs, scheduled_backlinks) =
        worker::schedule_tick(&ctx, 500, 900).await.map_err(|e| {
            tracing::error!(error = %e, "tick: schedule failed");
            ApiError::Internal
        })?;

    // The crawl-heavy work (process batch + warranty sweep + discovery + alert
    // delivery) runs under a HARD wall-clock budget. On the free/cron model the
    // tick is a single HTTP request, so it must always return well under the
    // platform's request ceiling — otherwise a slow batch (dead source pages,
    // hanging webhooks) runs for minutes and the proxy answers 502. Whatever
    // doesn't finish stays leased and is picked up by the next tick, so progress
    // is just spread across ticks rather than lost.
    const TICK_BUDGET: Duration = Duration::from_secs(70);
    let work = async {
        if !guards.may_process_jobs() {
            return (0usize, 0usize, 0usize, 0usize);
        }
        let processed = worker::tick(&ctx, 15).await.unwrap_or_else(|e| {
            tracing::error!(error = %e, "tick: process failed");
            0
        });
        let today = chrono::Utc::now().date_naive();
        let warranty_alerts = worker::engine::run_warranty_sweep(&state.pool, today, 14, 5000)
            .await
            .unwrap_or_else(|e| {
                tracing::error!(error = %e, "tick: warranty sweep failed");
                0
            });
        // Expansion discovery: a tiny bounded slice per tick (seeds reschedule).
        let discovered = worker::discover::discover_tick(&state.pool, &ctx.crawler, 1, 8)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(error = %e, "tick: discovery failed");
                0
            });
        // Fan out new notifications to webhook / Slack / email destinations
        // (SSRF-safe, bounded, idempotent).
        let delivered = worker::deliver::deliver_tick(&state.pool, &ctx.crawler, 10)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(error = %e, "tick: alert delivery failed");
                0
            });
        (processed, warranty_alerts, discovered, delivered)
    };

    let (partial, (processed, warranty_alerts, discovered, delivered)) =
        match tokio::time::timeout(TICK_BUDGET, work).await {
            Ok(counts) => (false, counts),
            Err(_) => {
                tracing::warn!("tick: budget exceeded; returning partial (continues next tick)");
                (true, (0, 0, 0, 0))
            }
        };

    // Reclaim any leases from this or a prior tick that stopped mid-flight.
    let _ = db::repositories::job_queue::reap(&state.pool, 900).await;

    Ok(Json(serde_json::json!({
        "scheduled_jobs": scheduled_jobs,
        "scheduled_backlinks": scheduled_backlinks,
        "processed": processed,
        "warranty_alerts": warranty_alerts,
        "discovered": discovered,
        "delivered": delivered,
        "partial": partial,
    })))
}
