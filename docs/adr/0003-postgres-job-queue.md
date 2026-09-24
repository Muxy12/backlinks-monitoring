# ADR-0003 — PostgreSQL as the durable job queue (no Redis)

- **Status:** Accepted
- **Date:** 2026-09-24
- **Spec refs:** §9, §10, §54, §57.6, §57.7

## Context

Monitoring must be asynchronous. We need a durable, retryable, observable queue. The reflex is
Redis + a worker library; the spec explicitly forbids that until Postgres is a *demonstrated*
bottleneck.

## Decision

A `jobs` table in Neon, claimed with `FOR UPDATE SKIP LOCKED`. No Redis, no SQS, no Kafka.

Claim query (one transaction, N rows):

```sql
UPDATE jobs SET
    status      = 'processing',
    locked_at   = now(),
    locked_by   = $1,
    attempts    = attempts + 1,
    updated_at  = now()
WHERE id IN (
    SELECT id FROM jobs
    WHERE status = 'pending'
      AND scheduled_at <= now()
    ORDER BY priority DESC, scheduled_at
    FOR UPDATE SKIP LOCKED
    LIMIT $2
)
RETURNING *;
```

Backed by a partial index on the hot predicate only:

```sql
CREATE INDEX jobs_claim_idx ON jobs (priority DESC, scheduled_at)
    WHERE status = 'pending';
```

Supporting decisions:

- **Wakeup is polling, not `LISTEN`/`NOTIFY`.** Neon's pooler does not support `LISTEN`/`NOTIFY`
  ([01 §2.2](../01-research-findings.md#22-features-that-do-not-work-through-the-neon-pooler)).
  The worker polls on a short interval with backoff when the queue is empty. At MVP volume the
  latency cost is irrelevant and the operational simplicity is worth a lot.
- **Stuck-job reaping.** A row `processing` with `locked_at < now() - lease_interval` is returned to
  `pending` (or moved to `dead` past `max_attempts`). This is how we survive a worker being killed
  mid-job — the alternative is jobs that silently vanish.
- **Idempotency is the handler's job, not the queue's.** At-least-once delivery is assumed. Every
  handler must tolerate re-execution; see [07](../07-job-queue.md#4-idempotency).
- **Retries** use exponential backoff with jitter, written into `scheduled_at` on failure.

## When we would revisit

Concrete triggers, so this is falsifiable rather than a vibe:

- Claim latency p99 > 100 ms at target throughput, or
- Queue depth persistently growing while workers are below CPU saturation, or
- `jobs` table bloat/vacuum pressure visibly affecting unrelated queries.

Until one of those is *measured*, Postgres stays. The metrics in [12](../12-observability.md)
(`queue_depth`, `job_age_seconds`, `claim_duration_ms`) exist specifically so this decision can be
re-litigated with data.

## Consequences

- One less piece of infrastructure, one less set of credentials, one less thing to be down.
- Jobs are visible in SQL. "Why didn't this backlink get checked" is a `SELECT`, not a Redis dump.
- Job state participates in the same transaction as domain writes where that matters (e.g. enqueue
  an import job in the same transaction that creates the `imports` row).
- Cost: the queue competes with application traffic for the same Neon compute. Mitigated by the
  worker using the direct endpoint and a separate, small pool.
