-- 0029 — trial lifecycle email idempotency ledger (docs/20 — trial UX)
-- One row per (workspace, email kind) that has been sent. The claim is an
-- INSERT ... ON CONFLICT DO NOTHING, so a reminder is sent at most once even
-- across retries, overlapping scans or multiple workers.

CREATE TABLE trial_email_log (
    organization_id uuid        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    kind            text        NOT NULL,
    sent_at         timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (organization_id, kind),
    CONSTRAINT trial_email_log_kind
        CHECK (kind IN ('started', 't_minus_3', 't_minus_1', 'expired'))
);
