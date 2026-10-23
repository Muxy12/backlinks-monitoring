-- 0045 — Spot Finder (Lot C): quality enrichment + score.
--
-- Stage 2 of the funnel: enrich only the shortlist of spots (stage 1 = the SERP
-- run, 0044) with authority / trust / traffic / keyword-breadth metrics from
-- DataForSEO (Backlinks summary + Labs), then compute a transparent, explainable
-- quality score. Metrics live on the spot row (1:1); they are nullable (NULL =
-- not yet enriched) and are reset when a SERP re-run replaces the spots.
-- `spot_enrich_runs` tracks one enrichment pass (status + cost).

ALTER TABLE spots
    ADD COLUMN IF NOT EXISTS domain_rank       int,     -- DataForSEO rank 0..1000 (authority)
    ADD COLUMN IF NOT EXISTS referring_domains int,     -- referring domains to this site
    ADD COLUMN IF NOT EXISTS spam_score        int,     -- backlink spam score 0..100 (lower better)
    ADD COLUMN IF NOT EXISTS organic_keywords  int,     -- how many keywords the site ranks for
    ADD COLUMN IF NOT EXISTS organic_traffic   bigint,  -- estimated monthly organic traffic
    ADD COLUMN IF NOT EXISTS quality_score     double precision, -- 0..100, explainable
    ADD COLUMN IF NOT EXISTS enriched_at        timestamptz;

CREATE TABLE IF NOT EXISTS spot_enrich_runs (
    id                uuid PRIMARY KEY,
    organization_id   uuid NOT NULL REFERENCES organizations ON DELETE CASCADE,
    project_id        uuid NOT NULL,
    status            text NOT NULL DEFAULT 'queued',  -- queued | running | done | error
    spots_enriched    int  NOT NULL DEFAULT 0,
    provider_queries  int  NOT NULL DEFAULT 0,         -- DataForSEO calls billed
    error             text,
    created_at        timestamptz NOT NULL DEFAULT now(),
    updated_at        timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT spot_enrich_runs_status_valid
        CHECK (status IN ('queued', 'running', 'done', 'error')),
    CONSTRAINT spot_enrich_runs_project_fk FOREIGN KEY (organization_id, project_id)
        REFERENCES projects (organization_id, id) ON DELETE CASCADE
);

CREATE TRIGGER spot_enrich_runs_set_updated_at
    BEFORE UPDATE ON spot_enrich_runs
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE INDEX IF NOT EXISTS spot_enrich_runs_project_idx
    ON spot_enrich_runs (project_id, created_at DESC);
