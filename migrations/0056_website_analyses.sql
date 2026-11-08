-- AI site analysis (refonte P2): the semantic profile of a user's own site —
-- theme, sub-themes, language, products/services, detected keywords and a short
-- summary. Produced by the AnalyzeSite job (heuristic baseline, upgraded to an
-- LLM profile when one is configured). One row per project (re-running upserts).
-- "Detected" keywords live here (AI/heuristic); "positioned" keywords (real
-- Google positions) come from GSC / SERP and are kept separate — never conflated.

CREATE TABLE IF NOT EXISTS website_analyses (
    id              uuid PRIMARY KEY,
    organization_id uuid NOT NULL REFERENCES organizations ON DELETE CASCADE,
    project_id      uuid NOT NULL,
    status          text NOT NULL DEFAULT 'queued',   -- queued | running | done | error
    source          text,                             -- heuristic | ai (what produced it)
    language        text,                             -- primary language (BCP-47-ish, e.g. 'fr')
    theme           text,                             -- main theme
    sub_themes      text[] NOT NULL DEFAULT '{}',
    products        text[] NOT NULL DEFAULT '{}',     -- products / services offered
    keywords        text[] NOT NULL DEFAULT '{}',     -- detected keywords (NOT verified positions)
    summary         text,                             -- one-paragraph profile
    edited          boolean NOT NULL DEFAULT false,   -- user corrected the AI result
    error           text,
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT website_analyses_status_valid
        CHECK (status IN ('queued', 'running', 'done', 'error')),
    CONSTRAINT website_analyses_source_valid
        CHECK (source IS NULL OR source IN ('heuristic', 'ai')),
    CONSTRAINT website_analyses_project_uq UNIQUE (project_id),
    CONSTRAINT website_analyses_project_fk FOREIGN KEY (organization_id, project_id)
        REFERENCES projects (organization_id, id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS website_analyses_org_idx ON website_analyses (organization_id);
