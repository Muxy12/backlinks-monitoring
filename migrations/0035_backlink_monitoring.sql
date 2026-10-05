-- 0035 selective monitoring: not every backlink is actively checked.
-- An explicit monitoring flag gates the scheduler; inventory-only links generate
-- ZERO recurring crawl (their next_check_at is NULL, which the scheduler already
-- excludes). Plus a few economic/commercial fields for value-at-risk. The
-- vocabularies (level/frequency/reason/commercial_model) are validated in the API
-- and kept in code, so they can evolve without another migration -- except where a
-- small, stable CHECK is cheap insurance.

ALTER TABLE backlinks
    ADD COLUMN monitoring_enabled    boolean NOT NULL DEFAULT false,
    ADD COLUMN monitoring_level      text NOT NULL DEFAULT 'inventory_only'
        CHECK (monitoring_level IN ('protected', 'tracked', 'inventory_only')),
    ADD COLUMN check_frequency       text
        CHECK (check_frequency IS NULL
               OR check_frequency IN ('daily', 'weekly', 'biweekly', 'monthly', 'manual')),
    ADD COLUMN monitoring_reason     text,
    -- True once a user manually sets monitoring, so the default policy never
    -- clobbers a deliberate choice.
    ADD COLUMN monitoring_overridden boolean NOT NULL DEFAULT false,
    ADD COLUMN commercial_model      text
        CHECK (commercial_model IS NULL
               OR commercial_model IN ('one_time', 'rental', 'recurring',
                                       'exchange', 'partnership', 'free', 'unknown')),
    ADD COLUMN order_reference       text,
    ADD COLUMN renewal_date          date,
    ADD COLUMN replacement_eligible  boolean;
-- (purchase_date already exists from 0005; it was unused and is now wired up.)

-- Scheduler gate: the due-index now also requires monitoring_enabled, so the
-- scan only ever touches links that are actually being monitored.
DROP INDEX IF EXISTS backlinks_due_idx;
CREATE INDEX backlinks_due_idx ON backlinks (next_check_at)
    WHERE archived_at IS NULL AND next_check_at IS NOT NULL AND monitoring_enabled;

-- Tab/segment filtering by level within an org (only the non-default values).
CREATE INDEX backlinks_monitoring_idx ON backlinks (organization_id, monitoring_level)
    WHERE monitoring_level <> 'inventory_only';

-- ── Policy-based migration of existing rows (conservative) ───────────────────
-- Monitor only links with an economic / contractual / strategic reason. Competitor
-- links, tier > 1, organic and unclassified links become inventory-only, and their
-- next_check_at is cleared so they stop generating crawls. This mirrors
-- crates/core/src/monitoring.rs so new and existing links are classified the same.
UPDATE backlinks AS b SET
    monitoring_enabled = d.enabled,
    monitoring_level   = d.level,
    monitoring_reason  = d.reason,
    next_check_at      = CASE WHEN d.enabled THEN b.next_check_at ELSE NULL END
FROM (
    SELECT id,
        (subject = 'own' AND tier = 1 AND (
            COALESCE(cost, 0) > 0
            OR warranty_type::text <> 'none'
            OR COALESCE(acquisition IN ('paid', 'exchange', 'partnership'), false)
        )) AS enabled,
        CASE
            WHEN subject <> 'own' OR tier <> 1 THEN 'inventory_only'
            WHEN COALESCE(cost, 0) > 0 OR warranty_type::text <> 'none'
                 OR acquisition = 'paid' THEN 'protected'
            WHEN acquisition IN ('exchange', 'partnership') THEN 'tracked'
            ELSE 'inventory_only'
        END AS level,
        CASE
            WHEN subject <> 'own' OR tier <> 1 THEN NULL
            WHEN COALESCE(cost, 0) > 0 THEN 'economic_value'
            WHEN warranty_type::text <> 'none' THEN 'guarantee'
            WHEN acquisition = 'paid' THEN 'paid_link'
            WHEN acquisition = 'exchange' THEN 'exchange'
            WHEN acquisition = 'partnership' THEN 'partnership'
            ELSE NULL
        END AS reason
    FROM backlinks
) AS d
WHERE b.id = d.id;
