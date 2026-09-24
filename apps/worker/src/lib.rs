//! Backlio worker (docs/07). The check engine plus the claim loop that drives
//! it. The loop claims jobs with FOR UPDATE SKIP LOCKED, dispatches by type,
//! and completes/retries/reaps — long-running, NOT on Vercel (ADR-0002).

pub mod engine;

use backlio_core::TransitionPolicy;
use crawler::CrawlerConfig;
use db::repositories::job_queue::{self, ClaimedJob};
use jobs::{BackoffPolicy, JobKind, RetryDecision, retry_delay};
use sqlx::PgPool;
use uuid::Uuid;

/// Everything the loop needs.
pub struct WorkerCtx {
    pub pool: PgPool,
    pub crawler: CrawlerConfig,
    pub policy: TransitionPolicy,
    pub backoff: BackoffPolicy,
    pub worker_id: String,
}

/// Claim and run up to `batch` jobs. Returns how many were processed (0 = idle).
pub async fn tick(ctx: &WorkerCtx, batch: i64) -> anyhow::Result<usize> {
    let claimed = job_queue::claim(&ctx.pool, &ctx.worker_id, batch).await?;
    let n = claimed.len();
    for job in claimed {
        run_one(ctx, job).await;
    }
    Ok(n)
}

async fn run_one(ctx: &WorkerCtx, job: ClaimedJob) {
    match dispatch(ctx, &job).await {
        Ok(()) => {
            if let Err(e) = job_queue::complete(&ctx.pool, job.id).await {
                tracing::error!(job_id = %job.id, error = %e, "failed to mark job complete");
            }
        }
        Err(err) => {
            // Jitter derived from the job id so retries of a batch don't align.
            let jitter = (job.id.as_u128() % 1000) as f64 / 1000.0;
            let retry_in =
                match retry_delay(&ctx.backoff, job.attempts, job.max_attempts, false, jitter) {
                    RetryDecision::Retry { delay } => Some(delay.num_seconds()),
                    RetryDecision::Dead => None,
                };
            tracing::warn!(job_id = %job.id, attempts = job.attempts, ?retry_in, error = %err, "job failed");
            let _ = job_queue::fail(&ctx.pool, job.id, &err.to_string(), retry_in).await;
        }
    }
}

async fn dispatch(ctx: &WorkerCtx, job: &ClaimedJob) -> anyhow::Result<()> {
    let kind: JobKind = job
        .job_type
        .parse()
        .map_err(|_| anyhow::anyhow!("unknown job_type {}", job.job_type))?;
    match kind {
        JobKind::CheckBacklinkNow => {
            let id = payload_uuid(&job.payload, "backlink_id")?;
            engine::process_backlink_check(&ctx.pool, &ctx.crawler, &ctx.policy, id).await?;
            Ok(())
        }
        // Other job types wire in as their handlers land (docs/07 §5).
        other => Err(anyhow::anyhow!("no handler for {other:?} yet")),
    }
}

fn payload_uuid(payload: &serde_json::Value, key: &str) -> anyhow::Result<Uuid> {
    let s = payload
        .get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("payload missing {key}"))?;
    Ok(Uuid::parse_str(s)?)
}
