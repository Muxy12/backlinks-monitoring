//! Worker entrypoint: claim → run → complete/retry, plus periodic reaping of
//! stuck jobs (docs/07). Long-running container (ADR-0002). Uses the DIRECT
//! Neon endpoint. Respects the env guard: it starts but stays idle where job
//! processing is disabled (docs/10 §1).

use backlio_core::TransitionPolicy;
use common::{AppEnv, Guards};
use crawler::CrawlerConfig;
use jobs::BackoffPolicy;
use std::str::FromStr;
use std::time::Duration;
use worker::{WorkerCtx, tick};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let app_env = std::env::var("APP_ENV")
        .ok()
        .and_then(|s| AppEnv::from_str(&s).ok())
        .unwrap_or(AppEnv::Development);
    let dev_opt_in = std::env::var("WORKER_ENABLE").as_deref() == Ok("true");
    let guards = Guards::new(app_env, dev_opt_in);
    if !guards.may_process_jobs() {
        tracing::warn!(env = %app_env, "job processing disabled for this environment — idling");
        // Stay up and healthy without doing dangerous work (docs/10 §1).
        loop {
            tokio::time::sleep(Duration::from_secs(3600)).await;
        }
    }

    let url = std::env::var("DATABASE_DIRECT_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .map_err(|_| "DATABASE_URL is required")?;
    let db = db::build_pool(&db::DbConfig::worker(&url, 16, None)).await?;

    let ctx = WorkerCtx {
        pool: db.pool,
        crawler: CrawlerConfig::default(),
        policy: TransitionPolicy::default(),
        backoff: BackoffPolicy::default(),
        worker_id: std::env::var("WORKER_ID").unwrap_or_else(|_| default_worker_id()),
    };

    let poll = Duration::from_millis(
        std::env::var("WORKER_POLL_INTERVAL_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1000),
    );
    let lease_secs: i64 = std::env::var("WORKER_JOB_LEASE_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(900);

    tracing::info!(worker_id = %ctx.worker_id, "worker started");
    let mut since_reap = std::time::Instant::now();
    loop {
        match tick(&ctx, 10).await {
            Ok(0) => tokio::time::sleep(poll * 5).await, // idle backoff
            Ok(n) => tracing::debug!(processed = n, "batch done"),
            Err(e) => {
                tracing::error!(error = %e, "tick failed");
                tokio::time::sleep(poll * 5).await;
            }
        }
        if since_reap.elapsed() > Duration::from_secs(60) {
            if let Err(e) = db::repositories::job_queue::reap(&ctx.pool, lease_secs).await {
                tracing::error!(error = %e, "reap failed");
            }
            since_reap = std::time::Instant::now();
        }
        tokio::time::sleep(poll).await;
    }
}

fn default_worker_id() -> String {
    format!("worker-{}", uuid::Uuid::now_v7())
}
