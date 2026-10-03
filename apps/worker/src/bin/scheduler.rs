//! One-shot scheduler tick, for cron-style invocation (ADR-0002 §scheduler):
//! enqueue due checks grouped by source URL, then exit. The worker also runs
//! this on a timer; the advisory lock makes concurrent runs safe.

use backlio_core::TransitionPolicy;
use crawler::CrawlerConfig;
use jobs::BackoffPolicy;
use worker::{WorkerCtx, schedule_tick};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let url = std::env::var("DATABASE_DIRECT_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .map_err(|_| "DATABASE_URL is required")?;
    let db = db::build_pool(&db::DbConfig::worker(&url, 4, None)).await?;
    let ctx = WorkerCtx {
        pool: db.pool,
        crawler: CrawlerConfig::default(),
        policy: TransitionPolicy::default(),
        backoff: BackoffPolicy::default(),
        worker_id: "scheduler".into(),
    };
    let batch: i64 = std::env::var("SCHEDULER_BATCH_SIZE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(500);
    let (jobs, n) = schedule_tick(&ctx, batch, 3600).await?;
    tracing::info!(jobs, backlinks = n, "scheduler tick complete");
    // Also flush any due trial lifecycle emails on the cron path (idempotent, so
    // running here and in the worker loop never double-sends).
    match worker::trial_emails_tick(&ctx).await {
        Ok(sent) => tracing::info!(sent, "trial email tick complete"),
        Err(e) => tracing::error!(error = %e, "trial email tick failed"),
    }
    Ok(())
}
