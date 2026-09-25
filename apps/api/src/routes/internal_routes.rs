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
        crawler: CrawlerConfig {
            check_target: true,
            ..CrawlerConfig::default()
        },
        policy: TransitionPolicy::default(),
        backoff: BackoffPolicy::default(),
        worker_id: "api-tick".to_string(),
    };

    // Enqueue due checks (safe in any env — advisory-locked, no crawling).
    let (scheduled_jobs, scheduled_backlinks) =
        worker::schedule_tick(&ctx, 500, 900).await.map_err(|e| {
            tracing::error!(error = %e, "tick: schedule failed");
            ApiError::Internal
        })?;

    // Process a batch + run the warranty sweep only where crawling is allowed.
    let (processed, warranty_alerts, discovered) = if guards.may_process_jobs() {
        let processed = worker::tick(&ctx, 25).await.map_err(|e| {
            tracing::error!(error = %e, "tick: process failed");
            ApiError::Internal
        })?;
        let today = chrono::Utc::now().date_naive();
        let alerts = worker::engine::run_warranty_sweep(&state.pool, today, 14, 5000)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "tick: warranty sweep failed");
                ApiError::Internal
            })?;
        // Expansion discovery: a tiny bounded slice per tick so the cron stays
        // well under its timeout (seeds are rescheduled and progress over time).
        let discovered = worker::discover::discover_tick(&state.pool, &ctx.crawler, 1, 8)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(error = %e, "tick: discovery failed");
                0
            });
        (processed, alerts, discovered)
    } else {
        (0, 0, 0)
    };

    // Reclaim any leases from a tick that died mid-flight (docs/07).
    let _ = db::repositories::job_queue::reap(&state.pool, 900).await;

    Ok(Json(serde_json::json!({
        "scheduled_jobs": scheduled_jobs,
        "scheduled_backlinks": scheduled_backlinks,
        "processed": processed,
        "warranty_alerts": warranty_alerts,
        "discovered": discovered,
    })))
}
