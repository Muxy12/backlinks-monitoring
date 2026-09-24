-- 0006 — backlink check history and immutable change events
-- See docs/03-database-schema.md §3.5, docs/08-backlink-state-machine.md §6
--
-- backlink_checks is the raw record of every check. backlink_events is the
-- meaningful-change record that the §30 timeline renders. Most checks produce
-- no event; that is the normal, healthy case and is why they are separate
-- tables rather than a flag on checks.

CREATE TABLE backlink_checks (
    id              uuid PRIMARY KEY,
    backlink_id     uuid NOT NULL,
    organization_id uuid NOT NULL,
    -- Shared by every backlink resolved from one deduped fetch (§27).
    -- This is what ties one HTTP request to the N backlinks it answered.
    crawl_id        uuid NOT NULL,
    checked_at      timestamptz NOT NULL DEFAULT now(),

    found         boolean         NOT NULL,
    status_after  backlink_status NOT NULL,
    source_status int,
    target_status int,

    anchor_text    text,
    rel_attributes text[],
    is_dofollow    boolean,
    final_url      text,
    redirect_chain text[],

    duration_ms int,
    bytes       int,
    error_kind  text,
    raw         jsonb,   -- full CrawlResult, for debugging

    CONSTRAINT backlink_checks_status_range
        CHECK ((source_status IS NULL OR source_status BETWEEN 100 AND 599)
           AND (target_status IS NULL OR target_status BETWEEN 100 AND 599)),
    CONSTRAINT backlink_checks_duration_non_negative
        CHECK (duration_ms IS NULL OR duration_ms >= 0),
    CONSTRAINT backlink_checks_bytes_non_negative
        CHECK (bytes IS NULL OR bytes >= 0),
    CONSTRAINT backlink_checks_backlink_fk FOREIGN KEY (organization_id, backlink_id)
        REFERENCES backlinks (organization_id, id) ON DELETE CASCADE
);

CREATE INDEX backlink_checks_bl_idx    ON backlink_checks (backlink_id, checked_at DESC);
CREATE INDEX backlink_checks_crawl_idx ON backlink_checks (crawl_id);
-- Retention sweep (docs/03 §3.5): full rows 90 days, then downsampled.
CREATE INDEX backlink_checks_age_idx   ON backlink_checks (checked_at);

-- Re-running a job must not write a second row for the same (backlink, crawl).
-- This is the table-level half of job idempotency (docs/07 §4).
CREATE UNIQUE INDEX backlink_checks_idempotency_uq
    ON backlink_checks (backlink_id, crawl_id);

-- ─── Immutable change events (§29) ──────────────────────────────────────────
-- Append-only. Nothing updates or deletes these; the current state in
-- `backlinks` is a cache of the latest truth, while these ARE the record.
-- Kept forever — this is the customer-visible history (§57.8).
CREATE TABLE backlink_events (
    id              uuid PRIMARY KEY,
    backlink_id     uuid NOT NULL,
    organization_id uuid NOT NULL,
    check_id        uuid REFERENCES backlink_checks ON DELETE SET NULL,
    event_type      event_type  NOT NULL,
    -- Snapshots, not references: they stay meaningful even if the backlink is
    -- later edited. That is the §30 requirement that history is never destroyed.
    previous_data   jsonb,
    current_data    jsonb,
    detected_at     timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT backlink_events_backlink_fk FOREIGN KEY (organization_id, backlink_id)
        REFERENCES backlinks (organization_id, id) ON DELETE CASCADE
);

CREATE INDEX backlink_events_bl_idx  ON backlink_events (backlink_id, detected_at DESC);
CREATE INDEX backlink_events_org_idx ON backlink_events (organization_id, detected_at DESC);
-- Dashboard "recent changes" (§34) filters by type within an org.
CREATE INDEX backlink_events_org_type_idx
    ON backlink_events (organization_id, event_type, detected_at DESC);
