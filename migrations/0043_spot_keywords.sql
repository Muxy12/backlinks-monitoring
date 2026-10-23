-- 0043 — Spot Finder (Lot A): keyword candidates detected from a project's OWN
-- site. The Spot Finder prospecting engine (find sites that rank for the user's
-- keywords, then score and cross-reference them with link-buying marketplaces)
-- starts from keywords. Rather than ask the user to type them, we crawl their
-- homepage (P0) and the pages one click away (P1) and extract the <h1> headings
-- as candidates. The user then reviews/edits/prunes the list before ANY paid
-- SERP lookup (Lot B), so this step has no external cost — it is a crawl of the
-- user's own site via the same SSRF-safe fetch as monitoring.
--
-- Two tables: `spot_keyword_scans` tracks one detection run (so the UI can show
-- queued/running/done/error), and `spot_keywords` holds the reviewable
-- candidates. Both are tenant-scoped via the composite FK to projects, so a
-- project delete cascades.

CREATE TABLE IF NOT EXISTS spot_keyword_scans (
    id                uuid PRIMARY KEY,
    organization_id   uuid NOT NULL REFERENCES organizations ON DELETE CASCADE,
    project_id        uuid NOT NULL,
    status            text NOT NULL DEFAULT 'queued',  -- queued | running | done | error
    pages_scanned     int  NOT NULL DEFAULT 0,
    h1_count          int  NOT NULL DEFAULT 0,
    keywords_found    int  NOT NULL DEFAULT 0,
    error             text,
    created_at        timestamptz NOT NULL DEFAULT now(),
    updated_at        timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT spot_keyword_scans_status_valid
        CHECK (status IN ('queued', 'running', 'done', 'error')),
    CONSTRAINT spot_keyword_scans_project_fk FOREIGN KEY (organization_id, project_id)
        REFERENCES projects (organization_id, id) ON DELETE CASCADE
);

CREATE TRIGGER spot_keyword_scans_set_updated_at
    BEFORE UPDATE ON spot_keyword_scans
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE INDEX IF NOT EXISTS spot_keyword_scans_project_idx
    ON spot_keyword_scans (project_id, created_at DESC);

CREATE TABLE IF NOT EXISTS spot_keywords (
    id                uuid PRIMARY KEY,
    organization_id   uuid NOT NULL REFERENCES organizations ON DELETE CASCADE,
    project_id        uuid NOT NULL,
    keyword           text NOT NULL,   -- display form, user-editable
    normalized        text NOT NULL,   -- lowercased dedup key within a project
    source            text NOT NULL DEFAULT 'h1',        -- h1 | manual
    origin_url        text,            -- the page an h1 candidate came from
    status            text NOT NULL DEFAULT 'candidate', -- candidate | kept | rejected
    created_at        timestamptz NOT NULL DEFAULT now(),
    updated_at        timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT spot_keywords_keyword_not_blank CHECK (length(btrim(keyword)) > 0),
    CONSTRAINT spot_keywords_source_valid CHECK (source IN ('h1', 'manual')),
    CONSTRAINT spot_keywords_status_valid
        CHECK (status IN ('candidate', 'kept', 'rejected')),
    CONSTRAINT spot_keywords_project_fk FOREIGN KEY (organization_id, project_id)
        REFERENCES projects (organization_id, id) ON DELETE CASCADE,
    CONSTRAINT spot_keywords_uq UNIQUE (project_id, normalized)
);

CREATE TRIGGER spot_keywords_set_updated_at
    BEFORE UPDATE ON spot_keywords
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE INDEX IF NOT EXISTS spot_keywords_project_idx
    ON spot_keywords (project_id, status, created_at DESC);
