-- 0042 outreach sending engine (BETA). Schema for the automated follow-up
-- sequences that email a campaign's prospects on a cadence (see
-- crates/core/src/outreach_schedule.rs) and stop the moment a prospect replies.
-- Additive, backfill-free — safe to hand-apply. The sending/dispatch code lands
-- in a later PR; this migration only provisions the tables.

-- One row per prospect per cadence step: what is queued/sent, when it is due,
-- and the provider's id for the send. `status` + `step` are code-validated; a
-- cheap CHECK guards the status vocabulary.
CREATE TABLE outreach_messages (
    id                  uuid        PRIMARY KEY,
    organization_id     uuid        NOT NULL REFERENCES organizations ON DELETE CASCADE,
    campaign_id         uuid        NOT NULL,
    prospect_id         uuid        NOT NULL,
    step                smallint    NOT NULL,
    status              text        NOT NULL DEFAULT 'queued'
        CONSTRAINT outreach_messages_status_chk
        CHECK (status IN ('queued', 'sent', 'skipped', 'bounced', 'failed', 'stopped')),
    scheduled_at        timestamptz NOT NULL,
    sent_at             timestamptz,
    to_email            citext      NOT NULL,
    subject             text,
    provider_message_id text,
    error               text,
    created_at          timestamptz NOT NULL DEFAULT now(),
    updated_at          timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT outreach_messages_step_pos CHECK (step >= 1),
    CONSTRAINT outreach_messages_prospect_fk
        FOREIGN KEY (organization_id, prospect_id)
        REFERENCES prospects (organization_id, id) ON DELETE CASCADE,
    CONSTRAINT outreach_messages_campaign_fk
        FOREIGN KEY (organization_id, campaign_id)
        REFERENCES campaigns (organization_id, id) ON DELETE CASCADE,
    -- Exactly one row per prospect per step (idempotent scheduling).
    CONSTRAINT outreach_messages_prospect_step_uq UNIQUE (prospect_id, step)
);
-- Dispatch pulls the queued steps that are due, oldest first.
CREATE INDEX outreach_messages_due_idx
    ON outreach_messages (scheduled_at)
    WHERE status = 'queued';
CREATE INDEX outreach_messages_prospect_idx ON outreach_messages (prospect_id);

-- Recipients who must never be emailed again by this org's outreach: an
-- unsubscribe click, a hard bounce, a complaint, or a manual add. Checked before
-- every send. Per-org — the suppression belongs to the tenant's relationship.
CREATE TABLE outreach_suppressions (
    organization_id uuid        NOT NULL REFERENCES organizations ON DELETE CASCADE,
    email           citext      NOT NULL,
    reason          text        NOT NULL DEFAULT 'unsubscribe'
        CONSTRAINT outreach_suppressions_reason_chk
        CHECK (reason IN ('unsubscribe', 'bounce', 'complaint', 'manual')),
    created_at      timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (organization_id, email)
);
