-- 0020 — Tier 1 / Tier 2 backlinks as graph distance (backlink-intelligence §5–§10)
--
-- A backlink is a graph edge. Its `tier` is its distance from the monitored root
-- domain: Tier 1 links point at our site; Tier 2 links point at a Tier-1 source
-- page (reinforcing it). No new edge table — we reuse backlinks and record the
-- distance plus, for Tier 2+, the parent edge it reinforces. Existing rows are
-- Tier 1 (the default), so nothing changes for them.

ALTER TABLE backlinks
    ADD COLUMN IF NOT EXISTS tier               smallint NOT NULL DEFAULT 1,
    ADD COLUMN IF NOT EXISTS parent_backlink_id uuid;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'backlinks_tier_positive') THEN
        ALTER TABLE backlinks ADD CONSTRAINT backlinks_tier_positive CHECK (tier >= 1);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'backlinks_parent_fk') THEN
        ALTER TABLE backlinks ADD CONSTRAINT backlinks_parent_fk
            FOREIGN KEY (organization_id, parent_backlink_id)
            REFERENCES backlinks (organization_id, id) ON DELETE CASCADE;
    END IF;
END $$;

-- Tier-2 links reinforcing a given Tier-1 edge; and tier-scoped listing.
CREATE INDEX IF NOT EXISTS backlinks_parent_idx
    ON backlinks (parent_backlink_id) WHERE parent_backlink_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS backlinks_org_tier_idx
    ON backlinks (organization_id, tier);
