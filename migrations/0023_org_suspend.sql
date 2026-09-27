-- 0023 — organization suspension (platform admin, migration 0022 lineage).
--
-- A suspended org is frozen: `authorize_org` refuses to mint a scope for it, so
-- every tenant-scoped route returns 403 for its members until it is unsuspended.
-- The platform admin backoffice is unaffected (it does not go through
-- `authorize_org`), so an operator can still see and unsuspend it. Reversible —
-- suspension is a timestamp, not a delete.

ALTER TABLE organizations
    ADD COLUMN IF NOT EXISTS suspended_at timestamptz;
