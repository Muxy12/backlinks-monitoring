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
use std::sync::atomic::{AtomicBool, Ordering};
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

/// In-process gate so only ONE tick runs at a time. On the free tier the API is
/// a single instance, so this fully serializes monitoring rounds: a second
/// overlapping call (a manual poke landing on top of the cron, say) returns
/// immediately instead of doubling the in-flight crawls and OOM-killing the
/// 512 MB instance. (Job claiming already uses SKIP LOCKED, so correctness never
/// depended on this — only memory safety does.)
static TICK_RUNNING: AtomicBool = AtomicBool::new(false);

/// Clears the gate on drop, so a panic or early return can't wedge it shut.
struct TickGuard;
impl Drop for TickGuard {
    fn drop(&mut self) {
        TICK_RUNNING.store(false, Ordering::SeqCst);
    }
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

    // Serialize ticks (memory safety — see TICK_RUNNING). If one is already in
    // flight, this call is a no-op rather than piling a second batch of crawls
    // on top. `_guard` clears the flag when this handler returns.
    let _guard =
        match TICK_RUNNING.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(_) => TickGuard,
            Err(_) => {
                return Ok(Json(
                    serde_json::json!({ "skipped": "another tick is already running" }),
                ));
            }
        };

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
            // Smaller body cap than the default 2 MiB: HTML parsing is synchronous
            // and CPU-bound, and on the free tier's ~0.1 shared CPU a big page can
            // block the runtime for tens of seconds (long enough that the budget
            // timer below can't even fire). 768 KiB keeps each parse short; the
            // link we monitor is virtually always in the first few hundred KiB.
            max_response_bytes: 768 * 1024,
            ..CrawlerConfig::default()
        },
        policy: TransitionPolicy::default(),
        backoff: BackoffPolicy::default(),
        worker_id: "api-tick".to_string(),
    };

    // Enqueue due checks (advisory-locked, no crawling). Bounded in batch and
    // time: on a cold Neon pooler enqueueing can be slow, and it runs OUTSIDE the
    // crawl budget below, so cap it so it can't dominate the request on its own.
    let (scheduled_jobs, scheduled_backlinks) = match tokio::time::timeout(
        Duration::from_secs(12),
        worker::schedule_tick(&ctx, 150, 900),
    )
    .await
    {
        Ok(Ok(counts)) => counts,
        Ok(Err(e)) => {
            tracing::error!(error = %e, "tick: schedule failed");
            (0, 0)
        }
        Err(_) => {
            tracing::warn!("tick: schedule timed out");
            (0, 0)
        }
    };

    // The crawl-heavy work (process batch + warranty sweep + discovery + alert
    // delivery) runs under a HARD wall-clock budget. On the free/cron model the
    // tick is a single HTTP request, so it must always return well under the
    // platform's request ceiling — empirically Render answers 502 once a request
    // runs past ~55s (observed under contention when a manual tick overlapped the
    // cron), so the budget is kept comfortably below that. Whatever doesn't
    // finish stays leased and is picked up by the next tick, so progress is just
    // spread across ticks rather than lost.
    const TICK_BUDGET: Duration = Duration::from_secs(30);
    let work = async {
        if !guards.may_process_jobs() {
            return (0usize, 0usize, 0usize, 0usize);
        }
        let processed = worker::tick(&ctx, 8).await.unwrap_or_else(|e| {
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

    // Roll up today's per-(org, project) backlink counts for the Overview growth
    // trend (migration 0037). Cheap, idempotent per day, DB-only — safe on every
    // tick regardless of the crawl budget or env guards.
    let snapshots = db::repositories::snapshots::roll_up_today(&state.pool)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, "tick: snapshot roll-up failed");
            0
        });

    // Pull one due Search Console property's daily Search Analytics into
    // gsc_daily (Post-Link Impact). One property per tick keeps the external
    // calls tiny; a property is re-synced at most ~daily (stale gate). Bounded so
    // it can't blow the request budget. Inert (returns 0) when GSC is unconfigured.
    let gsc_synced = tokio::time::timeout(
        Duration::from_secs(12),
        crate::gsc_sync::sync_due(&state.pool, 1),
    )
    .await
    .unwrap_or(0);

    // Reclaim any leases from this or a prior tick that stopped mid-flight.
    let _ = db::repositories::job_queue::reap(&state.pool, 900).await;

    Ok(Json(serde_json::json!({
        "scheduled_jobs": scheduled_jobs,
        "scheduled_backlinks": scheduled_backlinks,
        "processed": processed,
        "warranty_alerts": warranty_alerts,
        "discovered": discovered,
        "delivered": delivered,
        "snapshots": snapshots,
        "gsc_synced": gsc_synced,
        "partial": partial,
    })))
}
