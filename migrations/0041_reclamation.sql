-- 0041 link reclamation (BETA). Groundwork for auto-created "win back a lost
-- link" prospects. Two additive, backfill-free changes — safe to hand-apply:
--
--   1. A campaign can be a per-project SYSTEM campaign (system_kind) rather than
--      a user-created one — e.g. the standing "Link reclamation" pipeline that
--      the monitoring engine feeds. NULL = an ordinary, user-created campaign.
--   2. A prospect can point back to the backlink it is trying to reclaim
--      (origin_backlink_id), so the engine can auto-close it when the link is
--      restored (BacklinkRestored). NULL for gap/manual prospects.
--
-- Vocabularies stay code-validated (like 0034/0035/0036); only cheap, stable
-- guards live in SQL.

-- 1. System campaigns ---------------------------------------------------------
ALTER TABLE campaigns
    ADD COLUMN system_kind text
        CONSTRAINT campaigns_system_kind_chk
        CHECK (system_kind IS NULL OR system_kind IN ('reclamation'));

-- At most one system campaign of each kind per project, so the engine can
-- idempotently get-or-create the reclamation pipeline for a project.
CREATE UNIQUE INDEX campaigns_system_kind_uq
    ON campaigns (organization_id, project_id, system_kind)
    WHERE system_kind IS NOT NULL;

-- 2. Reclamation prospects ----------------------------------------------------
-- The backlink a reclamation prospect is trying to win back. A single-column
-- ON DELETE SET NULL keeps the prospect (and its outreach history) even if the
-- backlink row is later deleted. Tenant-safety is enforced in app code: the
-- backlink is always loaded within OrgScope before being linked here.
ALTER TABLE prospects
    ADD COLUMN origin_backlink_id uuid
        CONSTRAINT prospects_origin_backlink_fk
        REFERENCES backlinks (id) ON DELETE SET NULL;

-- Find the reclamation prospect for a backlink (auto-close on BacklinkRestored).
CREATE INDEX prospects_origin_backlink_idx
    ON prospects (origin_backlink_id)
    WHERE origin_backlink_id IS NOT NULL;
