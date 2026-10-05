-- 0039 Post-Link Impact analysis (BETA). One row per eligible backlink we're
-- observing after acquisition: a reliable T0 (and which timestamp it came from),
-- the target page, and an analysis status. Derived metrics (before/after,
-- change %) are NOT stored — they are computed on read from gsc_daily (the raw
-- series), so formulas can change without a backfill. Purely additive.

CREATE TABLE backlink_impact (
    id                   uuid        PRIMARY KEY,
    organization_id      uuid        NOT NULL REFERENCES organizations ON DELETE CASCADE,
    project_id           uuid        NOT NULL,
    backlink_id          uuid        NOT NULL REFERENCES backlinks (id) ON DELETE CASCADE,
    target_url_normalized text       NOT NULL,
    -- The acquisition anchor and where it came from (analytical integrity).
    t0                   timestamptz NOT NULL,
    t0_source            text        NOT NULL
        CHECK (t0_source IN ('publication', 'verification', 'acquisition', 'manual')),
    status               text        NOT NULL DEFAULT 'collecting',
    eligibility          text,
    created_at           timestamptz NOT NULL DEFAULT now(),
    updated_at           timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT backlink_impact_backlink_uq UNIQUE (backlink_id)
);
-- Attribution-collision lookup: other backlinks acquired for the same target.
CREATE INDEX backlink_impact_target_idx
    ON backlink_impact (organization_id, project_id, target_url_normalized);
