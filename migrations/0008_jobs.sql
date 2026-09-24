-- 0008 — durable job queue
-- See docs/07-job-queue.md, docs/adr/0003-postgres-job-queue.md
--
-- No Redis. Jobs are claimed with FOR UPDATE SKIP LOCKED so that N workers
-- never block each other and no job is ever handed to two workers.

CREATE TABLE jobs (
    id              uuid PRIMARY KEY,
    -- Nullable: system jobs (scheduler tick, retention sweep) have no tenant.
    -- Present on tenant work for observability and quota attribution only —
    -- the worker claims across all tenants (docs/05 §4).
    organization_id uuid,
    job_type        text        NOT NULL,
    payload         jsonb       NOT NULL DEFAULT '{}',
    status          job_status  NOT NULL DEFAULT 'pending',
    priority        int         NOT NULL DEFAULT 0,      -- higher runs first
    attempts        int         NOT NULL DEFAULT 0,
    max_attempts    int         NOT NULL DEFAULT 5,
    idempotency_key text,
    scheduled_at    timestamptz NOT NULL DEFAULT now(),
    locked_at       timestamptz,
    locked_by       text,                                 -- hostname + pid + boot uuid
    completed_at    timestamptz,
    failed_at       timestamptz,
    last_error      text,
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT jobs_job_type_not_blank CHECK (length(btrim(job_type)) > 0),
    CONSTRAINT jobs_attempts_non_negative CHECK (attempts >= 0),
    CONSTRAINT jobs_max_attempts_positive CHECK (max_attempts > 0),
    -- A processing row must carry its lease, or the reaper cannot reason
    -- about it (docs/07 §3.1).
    CONSTRAINT jobs_processing_has_lock
        CHECK (status <> 'processing' OR (locked_at IS NOT NULL AND locked_by IS NOT NULL))
);

CREATE TRIGGER jobs_set_updated_at
    BEFORE UPDATE ON jobs
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- ─── The claim index ────────────────────────────────────────────────────────
-- Partial, so it holds only claimable rows and stays small while `jobs`
-- accumulates completed history. Matches the ORDER BY of the claim query
-- exactly (docs/07 §2).
CREATE INDEX jobs_claim_idx ON jobs (priority DESC, scheduled_at)
    WHERE status = 'pending';

-- ─── The reaper index ───────────────────────────────────────────────────────
-- A worker OOM-killed mid-job leaves a row `processing` forever. Without this
-- sweep, worker crashes silently stop monitoring — the failure mode that is
-- hardest to notice (docs/07 §3.1).
CREATE INDEX jobs_stuck_idx ON jobs (locked_at) WHERE status = 'processing';

-- ─── Enqueue-side deduplication ─────────────────────────────────────────────
-- "Check now" pressed five times enqueues one job (§36, docs/07 §4).
CREATE UNIQUE INDEX jobs_idempotency_uq ON jobs (idempotency_key)
    WHERE idempotency_key IS NOT NULL AND status IN ('pending', 'processing');

-- Observability and per-tenant queue inspection (docs/12).
CREATE INDEX jobs_org_idx  ON jobs (organization_id, created_at DESC)
    WHERE organization_id IS NOT NULL;
CREATE INDEX jobs_type_idx ON jobs (job_type, status);

-- Dead jobs require a human; alerting reads this (docs/12 §3).
CREATE INDEX jobs_dead_idx ON jobs (failed_at DESC) WHERE status = 'dead';
