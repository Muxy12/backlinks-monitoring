-- 0050 — AI-era refactor, Phase 1b: acquisition usage counters for the quota gate.
--
-- See docs/product/21 §6 and 23 §2. The quota gate is checked at enqueue:
-- discovery/GEO work spends provider units, so each run is metered here and an
-- over-quota enqueue is refused (API 429) — it NEVER deletes data and NEVER
-- gates monitoring. `usage_records` (0010) already meters monitoring; this adds
-- the two acquisition counters it was missing.
--
-- Period bucket stays `(organization_id, period_start)`; the app writes the
-- current calendar month, so the quota resets each month.

ALTER TABLE usage_records
    ADD COLUMN IF NOT EXISTS discoveries int NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS geo_queries int NOT NULL DEFAULT 0;

-- Extend the non-negative guard to the new counters (the original
-- `usage_records_non_negative` covers the monitoring counters).
-- IDEMPOTENT: guarded so a prod where these counters (and the constraint) were
-- hand-applied before version 50 was recorded does not abort on "already exists".
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'usage_records_acquisition_non_negative'
    ) THEN
        ALTER TABLE usage_records
            ADD CONSTRAINT usage_records_acquisition_non_negative
                CHECK (discoveries >= 0 AND geo_queries >= 0);
    END IF;
END $$;
