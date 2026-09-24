-- 0005 — backlinks, the core domain entity
-- See docs/03-database-schema.md §3.4, docs/08-backlink-state-machine.md

CREATE TABLE backlinks (
    id              uuid PRIMARY KEY,
    organization_id uuid NOT NULL REFERENCES organizations ON DELETE CASCADE,
    project_id      uuid NOT NULL,
    campaign_id     uuid,
    publisher_id    uuid,

    -- ─── Source ─────────────────────────────────────────────────────────────
    -- *_normalized is produced by the same Rust normalizer the crawler uses
    -- (docs/06 §3.1) and written on insert/update. NOT a generated column:
    -- the rule (public suffix list, tracking-param stripping, IDN) is not
    -- expressible in SQL without duplicating logic that would then drift.
    source_url            text NOT NULL,
    source_url_normalized text NOT NULL,
    source_domain         text NOT NULL,   -- registrable domain, public-suffix aware

    -- ─── Target ─────────────────────────────────────────────────────────────
    target_url            text NOT NULL,
    target_url_normalized text NOT NULL,
    target_domain         text NOT NULL,

    -- ─── What the customer bought ───────────────────────────────────────────
    expected_anchor_text text,
    expected_rel_type    rel_type NOT NULL DEFAULT 'dofollow',

    -- ─── What we last actually saw ──────────────────────────────────────────
    detected_anchor_text    text,
    detected_target_url     text,
    detected_rel_attributes text[],
    source_http_status      int,
    target_http_status      int,
    source_canonical        text,
    source_robots_meta      text,

    status          backlink_status NOT NULL DEFAULT 'pending',
    previous_status backlink_status,

    -- ─── Commercial (§32, §33) ──────────────────────────────────────────────
    cost                numeric(14,2),
    currency            char(3),
    purchase_date       date,
    warranty_type       warranty_type NOT NULL DEFAULT 'none',
    warranty_value      int,     -- days or months, per warranty_type
    warranty_expiration date,    -- materialized on write so it can be indexed

    -- ─── Monitoring (docs/08 §7) ────────────────────────────────────────────
    first_checked_at          timestamptz,
    last_checked_at           timestamptz,
    next_check_at             timestamptz,
    last_seen_at              timestamptz,
    failure_count             int NOT NULL DEFAULT 0,
    consecutive_failure_count int NOT NULL DEFAULT 0,

    notes       text,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now(),
    archived_at timestamptz,

    CONSTRAINT backlinks_source_url_not_blank CHECK (length(btrim(source_url)) > 0),
    CONSTRAINT backlinks_target_url_not_blank CHECK (length(btrim(target_url)) > 0),
    CONSTRAINT backlinks_http_status_range
        CHECK ((source_http_status IS NULL OR source_http_status BETWEEN 100 AND 599)
           AND (target_http_status IS NULL OR target_http_status BETWEEN 100 AND 599)),
    CONSTRAINT backlinks_counts_non_negative
        CHECK (failure_count >= 0 AND consecutive_failure_count >= 0),
    CONSTRAINT backlinks_cost_non_negative CHECK (cost IS NULL OR cost >= 0),
    CONSTRAINT backlinks_cost_currency     CHECK (cost IS NULL OR currency IS NOT NULL),
    CONSTRAINT backlinks_currency_format   CHECK (currency IS NULL OR currency ~ '^[A-Z]{3}$'),
    -- A 'days'/'months' warranty without a value cannot be evaluated (§32).
    -- The IS NOT NULL is load-bearing: `warranty_value > 0` yields NULL when
    -- the value is NULL, and a CHECK that evaluates to NULL is treated as
    -- SATISFIED. Without it this constraint silently permits exactly the row
    -- it exists to reject.
    CONSTRAINT backlinks_warranty_value
        CHECK (warranty_type NOT IN ('days', 'months')
               OR (warranty_value IS NOT NULL AND warranty_value > 0)),
    -- Archived links leave the scheduler entirely (docs/08 §7).
    CONSTRAINT backlinks_archived_not_scheduled
        CHECK (archived_at IS NULL OR next_check_at IS NULL),

    CONSTRAINT backlinks_org_id_uq UNIQUE (organization_id, id),
    CONSTRAINT backlinks_project_fk FOREIGN KEY (organization_id, project_id)
        REFERENCES projects (organization_id, id) ON DELETE CASCADE,
    CONSTRAINT backlinks_campaign_fk FOREIGN KEY (organization_id, campaign_id)
        REFERENCES campaigns (organization_id, id) ON DELETE SET NULL,
    CONSTRAINT backlinks_publisher_fk FOREIGN KEY (organization_id, publisher_id)
        REFERENCES publishers (organization_id, id) ON DELETE SET NULL
);

CREATE TRIGGER backlinks_set_updated_at
    BEFORE UPDATE ON backlinks
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- ─── The scheduler index — the most important one in the system ─────────────
-- Partial, so it contains only rows that can ever be due. Archived links and
-- links with no schedule are physically absent, keeping it small as the table
-- grows into the millions. The §11 scheduler query is an index range scan with
-- a LIMIT, never a table scan. (docs/03 §4.1, docs/07 §6)
CREATE INDEX backlinks_due_idx
    ON backlinks (next_check_at)
    WHERE archived_at IS NULL AND next_check_at IS NOT NULL;

-- ─── Tenant-scoped list / filter indexes (§12, §35) ─────────────────────────
-- Every one leads with organization_id, because every tenant query filters on
-- it first (docs/05). An index that does not lead with the tenant column is
-- useless to us.
CREATE INDEX backlinks_org_status_idx    ON backlinks (organization_id, status);
CREATE INDEX backlinks_org_project_idx   ON backlinks (organization_id, project_id);
CREATE INDEX backlinks_org_created_idx   ON backlinks (organization_id, created_at DESC);
CREATE INDEX backlinks_project_status_idx ON backlinks (project_id, status);
CREATE INDEX backlinks_campaign_idx      ON backlinks (campaign_id)  WHERE campaign_id IS NOT NULL;
CREATE INDEX backlinks_publisher_idx     ON backlinks (publisher_id) WHERE publisher_id IS NOT NULL;
CREATE INDEX backlinks_source_domain_idx ON backlinks (source_domain);

-- Dedup grouping for the crawler (§27, docs/06 §4.1, docs/07 §6.1).
-- This is what turns 20 customers monitoring one page into one HTTP request.
CREATE INDEX backlinks_source_norm_idx ON backlinks (source_url_normalized)
    WHERE archived_at IS NULL;

-- ─── Attention & warranty (§33, §34) ────────────────────────────────────────
CREATE INDEX backlinks_attention_idx
    ON backlinks (organization_id, status)
    WHERE archived_at IS NULL
      AND status IN ('lost', 'changed', 'nofollow',
                     'source_error', 'target_error', 'redirected');

CREATE INDEX backlinks_warranty_idx
    ON backlinks (organization_id, warranty_expiration)
    WHERE warranty_expiration IS NOT NULL AND archived_at IS NULL;

-- ─── Duplicate prevention (§38) ─────────────────────────────────────────────
-- The same link, twice, in one project. Partial on archived_at so a link can
-- be archived and later re-added.
CREATE UNIQUE INDEX backlinks_dedupe_uq
    ON backlinks (project_id, source_url_normalized, target_url_normalized)
    WHERE archived_at IS NULL;
