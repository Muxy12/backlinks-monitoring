-- 0052 — AI-era refactor, Phase 2: the GEO pillar ("cited by AI").
--
-- See docs/product/21 §4.2. The net-new pillar: track which sources an AI
-- assistant cites for a project's queries (DataForSEO AI Overviews), and join
-- the cited domains to buyable availability so we can say which AI-cited sources
-- are actually *placeable*. Additive; nothing else is touched. The CaptureGeo
-- job, repos and routes that populate these land in the following sub-PRs.
--
-- Shape: geo_queries 1─N geo_captures 1─N geo_citations;
-- geo_citations.cited_domain → geo_spot_links → (spots | marketplace_offers).

-- The GEO queries we track for a project (what we ask the AI engines).
CREATE TABLE IF NOT EXISTS geo_queries (
    id              uuid PRIMARY KEY,
    organization_id uuid NOT NULL REFERENCES organizations ON DELETE CASCADE,
    project_id      uuid NOT NULL,
    query           text NOT NULL,
    locale          text NOT NULL DEFAULT 'en',
    engine          text NOT NULL DEFAULT 'ai_overview',  -- ai_overview | perplexity | …
    enabled         boolean NOT NULL DEFAULT true,
    created_at      timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT geo_queries_query_not_blank CHECK (length(btrim(query)) > 0),
    CONSTRAINT geo_queries_project_fk FOREIGN KEY (organization_id, project_id)
        REFERENCES projects (organization_id, id) ON DELETE CASCADE,
    CONSTRAINT geo_queries_uq UNIQUE (project_id, query, locale, engine)
);

CREATE INDEX IF NOT EXISTS geo_queries_project_idx
    ON geo_queries (project_id, enabled);

-- One engine answer snapshot for a tracked query (a capture = one CaptureGeo run
-- for that query). `cache_ref` points at the provider_cache row it was read from
-- (for provenance), NULL when freshly fetched.
CREATE TABLE IF NOT EXISTS geo_captures (
    id              uuid PRIMARY KEY,
    organization_id uuid NOT NULL REFERENCES organizations ON DELETE CASCADE,
    project_id      uuid NOT NULL,
    geo_query_id    uuid NOT NULL REFERENCES geo_queries (id) ON DELETE CASCADE,
    engine          text NOT NULL,
    captured_at     timestamptz NOT NULL DEFAULT now(),
    cache_ref       text,

    CONSTRAINT geo_captures_project_fk FOREIGN KEY (organization_id, project_id)
        REFERENCES projects (organization_id, id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS geo_captures_query_idx
    ON geo_captures (geo_query_id, captured_at DESC);

-- The sources an answer cited, in order (rank 1 = first cited).
CREATE TABLE IF NOT EXISTS geo_citations (
    id              uuid PRIMARY KEY,
    organization_id uuid NOT NULL REFERENCES organizations ON DELETE CASCADE,
    project_id      uuid NOT NULL,
    geo_query_id    uuid NOT NULL REFERENCES geo_queries (id) ON DELETE CASCADE,
    capture_id      uuid NOT NULL REFERENCES geo_captures (id) ON DELETE CASCADE,
    cited_url       text NOT NULL,
    cited_domain    text NOT NULL,
    rank            int,
    captured_at     timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT geo_citations_project_fk FOREIGN KEY (organization_id, project_id)
        REFERENCES projects (organization_id, id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS geo_citations_capture_idx
    ON geo_citations (capture_id, rank);
CREATE INDEX IF NOT EXISTS geo_citations_domain_idx
    ON geo_citations (project_id, cited_domain);

-- Joins a cited domain to buyable availability: is this AI-cited source one we
-- can actually place a link on (an open spot or a marketplace offer)? Recomputed
-- after each capture. `placeable` drives the "actionable" filter in the UI/API.
CREATE TABLE IF NOT EXISTS geo_spot_links (
    id                   uuid PRIMARY KEY,
    organization_id      uuid NOT NULL REFERENCES organizations ON DELETE CASCADE,
    project_id           uuid NOT NULL,
    cited_domain         text NOT NULL,
    spot_id              uuid REFERENCES spots (id) ON DELETE SET NULL,
    marketplace_offer_id uuid REFERENCES marketplace_offers (id) ON DELETE SET NULL,
    placeable            boolean NOT NULL DEFAULT false,
    reason               text,
    updated_at           timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT geo_spot_links_project_fk FOREIGN KEY (organization_id, project_id)
        REFERENCES projects (organization_id, id) ON DELETE CASCADE,
    CONSTRAINT geo_spot_links_uq UNIQUE (project_id, cited_domain)
);

CREATE INDEX IF NOT EXISTS geo_spot_links_placeable_idx
    ON geo_spot_links (project_id, placeable);
