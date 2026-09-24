# 07 — Job queue and scheduler

**Spec refs:** §9, §10, §11, §36, §38, §57.4, §57.5
**Crate:** `crates/jobs`. **Decision:** [ADR-0003](adr/0003-postgres-job-queue.md).

---

## 1. Table

```sql
CREATE TABLE jobs (
    id              uuid PRIMARY KEY,
    organization_id uuid,                            -- nullable: system jobs have no tenant
    job_type        text        NOT NULL,
    payload         jsonb       NOT NULL DEFAULT '{}',
    status          job_status  NOT NULL DEFAULT 'pending',
    priority        int         NOT NULL DEFAULT 0,  -- higher runs first
    attempts        int         NOT NULL DEFAULT 0,
    max_attempts    int         NOT NULL DEFAULT 5,
    idempotency_key text,
    scheduled_at    timestamptz NOT NULL DEFAULT now(),
    locked_at       timestamptz,
    locked_by       text,
    completed_at    timestamptz,
    failed_at       timestamptz,
    last_error      text,
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now()
);

-- The claim index. Partial, so it holds only claimable rows and stays small
-- while `jobs` accumulates completed history.
CREATE INDEX jobs_claim_idx ON jobs (priority DESC, scheduled_at)
    WHERE status = 'pending';

-- The reaper index.
CREATE INDEX jobs_stuck_idx ON jobs (locked_at)
    WHERE status = 'processing';

-- Deduplication: "check now" pressed five times enqueues one job.
CREATE UNIQUE INDEX jobs_idempotency_uq ON jobs (idempotency_key)
    WHERE idempotency_key IS NOT NULL AND status IN ('pending','processing');

CREATE INDEX jobs_org_idx ON jobs (organization_id, created_at DESC);
```

Statuses are exactly §10's: `pending`, `processing`, `completed`, `failed`, `dead`.
`failed` is retryable (it will return to `pending` with a backoff); `dead` has exhausted
`max_attempts` and requires a human.

---

## 2. Claiming

```sql
UPDATE jobs SET
    status     = 'processing',
    locked_at  = now(),
    locked_by  = $1,          -- worker id: hostname + pid + boot uuid
    attempts   = attempts + 1,
    updated_at = now()
WHERE id IN (
    SELECT id FROM jobs
    WHERE status = 'pending' AND scheduled_at <= now()
    ORDER BY priority DESC, scheduled_at
    FOR UPDATE SKIP LOCKED
    LIMIT $2
)
RETURNING *;
```

`SKIP LOCKED` is what makes this safe with N workers: a row locked by another worker's transaction
is stepped over rather than waited on. No worker ever blocks another, and no job is ever handed to
two workers.

**Polling, not `LISTEN`/`NOTIFY`** — Neon's pooler does not support `LISTEN`/`NOTIFY`
([01 §2.2](01-research-findings.md#22-features-that-do-not-work-through-the-neon-pooler)), and the
worker's direct connection could use it but we would then have two wakeup paths to reason about.
The worker polls every 1 s when work was found, backing off to 5 s when the queue is empty.

---

## 3. Completion, retry, death

```rust
match handler.run(&job).await {
    Ok(()) => complete(job.id).await?,                       // status = completed
    Err(e) if e.is_permanent() => kill(job.id, &e).await?,   // status = dead, no retry
    Err(e) if job.attempts >= job.max_attempts
                      => kill(job.id, &e).await?,            // status = dead
    Err(e) => retry(job.id, backoff(job.attempts), &e).await?, // status = pending, future scheduled_at
}
```

`backoff(n) = min(base * 2^n, 1h) ± full jitter`. Jitter matters: without it, a Neon blip fails
500 jobs simultaneously and they all retry simultaneously, reproducing the outage.

**Permanent vs transient** is a property of the error, decided by the handler. A malformed payload
is permanent — retrying it five times is five guaranteed failures. A connection timeout is
transient. Handlers that cannot tell default to transient.

### 3.1 Reaping stuck jobs

A worker that is OOM-killed mid-job leaves a row `processing` forever. The reaper runs every 60 s:

```sql
UPDATE jobs SET status = 'pending', locked_at = NULL, locked_by = NULL
WHERE status = 'processing' AND locked_at < now() - $1::interval   -- lease, default 15 min
  AND attempts < max_attempts;
```

Rows past `max_attempts` go to `dead` instead. Without this, worker crashes silently stop
monitoring for the affected backlinks — the failure mode that is hardest to notice.

---

## 4. Idempotency

At-least-once delivery is assumed (§10). A job **will** occasionally run twice: reaped after a
timeout that was actually just slow, or retried after a commit whose ack was lost. Every handler
must be safe under re-execution.

| Job | How re-execution is made safe |
|---|---|
| `check_source_page` | Writes a `backlink_checks` row keyed by `crawl_id`; a repeat with the same `crawl_id` is an upsert no-op. State transition recomputed from the same inputs yields the same result. |
| `import_rows` | `import_rows.UNIQUE (import_id, row_number)` + `backlinks_dedupe_uq`. A re-run skips rows already carrying a `backlink_id`. |
| `send_notification` | `notifications` carries `UNIQUE (dedupe_key)` where the key is `(backlink_id, event_type, detected_at)`. |
| `stripe_webhook` | `stripe_events (event_id PRIMARY KEY)`. Insert conflict → already processed → return 200. |
| `schedule_due_checks` | Wrapped in `pg_advisory_xact_lock`; a concurrent run finds nothing due because the first already moved `next_check_at`. |

`idempotency_key` on the table handles the *enqueue* side — pressing "Check now" five times
(§36) produces one pending job, because the partial unique index rejects the duplicates.

---

## 5. Job types

| `job_type` | Priority | Payload | Notes |
|---|---|---|---|
| `check_source_page` | 0 | `{ source_url, backlink_ids[] }` | The deduped unit of work (§27). One fetch, N evaluations. |
| `check_backlink_now` | **100** | `{ backlink_id }` | User-triggered (§36). Jumps the queue; carries `idempotency_key`. |
| `check_target_url` | 0 | `{ target_url, backlink_ids[] }` | Deduped target checks. |
| `schedule_due_checks` | 50 | `{}` | The scheduler tick. |
| `process_import` | 20 | `{ import_id }` | Parses the upload, fans out to `import_rows_chunk`. |
| `import_rows_chunk` | 20 | `{ import_id, from, to }` | Bounded chunks so one huge CSV cannot monopolize a worker. |
| `send_notification` | 30 | `{ notification_id }` | |
| `digest_notifications` | 10 | `{ organization_id, window }` | Batching (§37). |
| `check_warranties` | 10 | `{}` | Daily; emits warranty-expiry alerts (§32). |
| `recompute_usage` | 10 | `{ organization_id }` | Plan-limit metering (§39). |
| `retention_downsample` | -10 | `{}` | Nightly `backlink_checks` compaction. |

`check_backlink_now` at priority 100 is the only user-facing latency path. Everything else is
background, which is the §9 rule: a user action **enqueues**, it never crawls inline.

---

## 6. Scheduler (§11)

Runs from two places, both safe to fire simultaneously: a Vercel Cron hitting
`POST /internal/scheduler/tick`, and the worker's own timer
([ADR-0002](adr/0002-api-and-worker-hosting.md#the-scheduler-vercel-cron--api--enqueue)).

```rust
// Transaction-scoped, NOT pg_advisory_lock — session-level advisory locks do not
// work through Neon's transaction pooler. See 01 §2.2.
sqlx::query!("SELECT pg_advisory_xact_lock($1)", SCHEDULER_LOCK_KEY)
    .execute(&mut *tx).await?;
```

Then, in **bounded batches** (§11 — never load millions of due backlinks into memory):

```sql
WITH due AS (
    SELECT id, source_url_normalized
    FROM backlinks
    WHERE archived_at IS NULL
      AND next_check_at IS NOT NULL
      AND next_check_at <= now()
    ORDER BY next_check_at
    LIMIT 500                       -- SCHEDULER_BATCH_SIZE
    FOR UPDATE SKIP LOCKED
)
SELECT source_url_normalized, array_agg(id) AS backlink_ids
FROM due
GROUP BY source_url_normalized;
```

This is one index range scan over `backlinks_due_idx`
([03 §4.1](03-database-schema.md#41-the-scheduler-index--the-most-important-one-in-the-system)),
stopping at 500 rows regardless of table size.

Then, **in the same transaction**:

1. Insert one `check_source_page` job per group.
2. `UPDATE backlinks SET next_check_at = next_check_at + frequency` for the claimed rows.

Both writes commit together. This is the important detail: if the enqueue committed and the
reschedule did not, the next tick would re-enqueue the same backlinks and the queue would grow
without bound. One transaction makes that impossible.

The tick repeats until a batch comes back empty or a wall-clock budget (30 s) expires, so a large
backlog drains across ticks rather than in one unbounded run.

### 6.1 Grouping is what makes the economics work

Grouping by `source_url_normalized` **before** enqueueing is the §27 optimization expressed
structurally: 20 customers monitoring `example.com/article` produce **one** job carrying 20 backlink
ids, not 20 jobs. The dedup ratio is a tracked metric ([12](12-observability.md)); if it drifts
toward 1.0, the cost model has broken and we want to know immediately.
