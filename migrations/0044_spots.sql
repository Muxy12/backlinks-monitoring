-- 0044 — Spot Finder (Lot B): SERP engine + spots.
--
-- The kept keywords (0043) feed a SERP lookup: for each keyword we ask who ranks
-- (DataForSEO, paid, env-gated), aggregate the ranking domains, and surface the
-- ones that rank for the user's keywords as "spots" — relevance-first link
-- prospects. Three tables:
--   * spot_search_runs — one SERP run (status + cost: queries actually spent),
--   * spots            — the aggregated prospects for a project,
--   * serp_cache       — org-scoped cache so re-running doesn't re-pay for the
--                        same keyword+locale within a freshness window.
-- Runs and spots are tenant-scoped via the composite FK to projects (cascade);
-- the cache is scoped to the organization (shared across its projects/runs).

CREATE TABLE IF NOT EXISTS spot_search_runs (
    id                uuid PRIMARY KEY,
    organization_id   uuid NOT NULL REFERENCES organizations ON DELETE CASCADE,
    project_id        uuid NOT NULL,
    status            text NOT NULL DEFAULT 'queued',  -- queued | running | done | error
    locale            text NOT NULL DEFAULT 'fr',      -- fr | en
    keywords_queried  int  NOT NULL DEFAULT 0,
    serp_queries      int  NOT NULL DEFAULT 0,         -- cache-miss queries actually billed
    spots_found       int  NOT NULL DEFAULT 0,
    error             text,
    created_at        timestamptz NOT NULL DEFAULT now(),
    updated_at        timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT spot_search_runs_status_valid
        CHECK (status IN ('queued', 'running', 'done', 'error')),
    CONSTRAINT spot_search_runs_locale_valid CHECK (locale IN ('fr', 'en')),
    CONSTRAINT spot_search_runs_project_fk FOREIGN KEY (organization_id, project_id)
        REFERENCES projects (organization_id, id) ON DELETE CASCADE
);

CREATE TRIGGER spot_search_runs_set_updated_at
    BEFORE UPDATE ON spot_search_runs
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE INDEX IF NOT EXISTS spot_search_runs_project_idx
    ON spot_search_runs (project_id, created_at DESC);

CREATE TABLE IF NOT EXISTS spots (
    id                uuid PRIMARY KEY,
    organization_id   uuid NOT NULL REFERENCES organizations ON DELETE CASCADE,
    project_id        uuid NOT NULL,
    domain            text NOT NULL,   -- registrable host (dedup key within a project)
    keyword_count     int  NOT NULL DEFAULT 0,   -- how many of the project's keywords it ranks for
    best_position     int,             -- best (lowest) organic rank across those keywords
    avg_position      double precision,
    example_url       text,            -- a page of this domain that ranks
    example_keyword   text,            -- a keyword it ranks for
    created_at        timestamptz NOT NULL DEFAULT now(),
    updated_at        timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT spots_domain_not_blank CHECK (length(btrim(domain)) > 0),
    CONSTRAINT spots_project_fk FOREIGN KEY (organization_id, project_id)
        REFERENCES projects (organization_id, id) ON DELETE CASCADE,
    CONSTRAINT spots_uq UNIQUE (project_id, domain)
);

CREATE TRIGGER spots_set_updated_at
    BEFORE UPDATE ON spots
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE INDEX IF NOT EXISTS spots_project_idx
    ON spots (project_id, keyword_count DESC);

CREATE TABLE IF NOT EXISTS serp_cache (
    id                  uuid PRIMARY KEY,
    organization_id     uuid NOT NULL REFERENCES organizations ON DELETE CASCADE,
    keyword_normalized  text NOT NULL,
    locale              text NOT NULL,
    results             jsonb NOT NULL,   -- [{url,domain,rank,title}]
    fetched_at          timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT serp_cache_uq UNIQUE (organization_id, keyword_normalized, locale)
);
