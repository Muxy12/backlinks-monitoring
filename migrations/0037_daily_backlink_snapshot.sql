-- 0037 daily backlink snapshot: a tiny once-a-day roll-up per (org, project) that
-- powers the Overview's Backlink-Growth trend. There is no other historical
-- time-series in the schema; recomputing growth from events on every dashboard
-- load would be expensive, so the scheduler tick upserts today's counts here and
-- the Overview just reads them. Purely additive.

CREATE TABLE daily_backlink_snapshot (
    organization_id   uuid        NOT NULL REFERENCES organizations ON DELETE CASCADE,
    project_id        uuid        NOT NULL,
    day               date        NOT NULL,
    live              int         NOT NULL DEFAULT 0,
    lost              int         NOT NULL DEFAULT 0,
    needs_attention   int         NOT NULL DEFAULT 0,
    referring_domains int         NOT NULL DEFAULT 0,
    monitored         int         NOT NULL DEFAULT 0,
    protected_count   int         NOT NULL DEFAULT 0,
    created_at        timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (organization_id, project_id, day),
    CONSTRAINT dbs_project_fk FOREIGN KEY (organization_id, project_id)
        REFERENCES projects (organization_id, id) ON DELETE CASCADE
);
-- Read path: a project/org's series within a date range, oldest-first.
CREATE INDEX daily_backlink_snapshot_org_day_idx ON daily_backlink_snapshot (organization_id, day);

-- The Overview's "links added" count buckets by first-seen; back it with an
-- expression index matching the dashboard's own first-seen predicate.
CREATE INDEX backlinks_first_seen_idx
    ON backlinks ((COALESCE(first_checked_at, created_at)))
    WHERE archived_at IS NULL AND subject = 'own' AND tier = 1;
