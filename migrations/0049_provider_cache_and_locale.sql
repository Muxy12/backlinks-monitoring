-- 0049 — AI-era refactor, Phase 0: mutualised provider cache + project SERP locale.
--
-- See docs/product/21-ai-link-intelligence-architecture.md §4.2/§6 and
-- docs/product/23-jobs-and-pipeline.md §4. This is the SCHEMA half of Phase 0;
-- the cache wrapper that reads/writes it and the enqueue-time quota checks are
-- wired in Phase 1. It is additive and touches nothing in the monitoring
-- (retention) layer.
--
-- Two things:
--   1. provider_cache — ONE cross-tenant, PII-free cache in front of every paid
--      provider call (SERP, domain metrics, AI Overviews). A hit costs 0
--      provider units and is reused across every workspace: this is the margin
--      lever (§6). It generalises the existing org-scoped `serp_cache` (0044),
--      which stays until Phase 1 migrates its rows under this table.
--   2. projects.serp_locale — the project's default SERP/GEO locale, so
--      discovery and GEO run in the right market (overridable per API call).

-- 1) Mutualised provider cache ------------------------------------------------
-- Keyed by (provider, cache_key) where cache_key is a canonical hash of the
-- request (e.g. serp:{sha(keyword)}:{locale}, metrics:{domain},
-- ai_overview:{sha(query)}:{locale}). Reads are tenant-agnostic by design, so
-- NEVER store anything tenant-identifying or personal in `payload`.
CREATE TABLE IF NOT EXISTS provider_cache (
    id          uuid PRIMARY KEY,
    provider    text        NOT NULL,   -- 'dfs_serp' | 'dfs_metrics' | 'dfs_ai_overview'
    cache_key   text        NOT NULL,   -- canonical hash of {kind, query|domain, locale}
    locale      text,                   -- NULL when not locale-scoped (e.g. metrics)
    payload     jsonb       NOT NULL,   -- the provider response, PII-free
    cost_units  int         NOT NULL DEFAULT 0,   -- provider units this fetch spent
    hits        int         NOT NULL DEFAULT 0,   -- times served from cache since fetch
    fetched_at  timestamptz NOT NULL DEFAULT now(),
    expires_at  timestamptz NOT NULL,   -- TTL per kind (SERP 24h, metrics 7d, AI 24–72h)

    CONSTRAINT provider_cache_uq UNIQUE (provider, cache_key),
    CONSTRAINT provider_cache_non_negative CHECK (cost_units >= 0 AND hits >= 0),
    CONSTRAINT provider_cache_ttl_sane CHECK (expires_at >= fetched_at)
);

-- Freshness lookups and the expiry sweep both read expires_at.
CREATE INDEX IF NOT EXISTS provider_cache_expiry_idx
    ON provider_cache (provider, expires_at);

-- 2) Project default SERP/GEO locale -----------------------------------------
-- BCP-47-ish tag (e.g. 'en', 'en-US', 'fr'). Discovery/GEO read this; an API
-- call may override it per request (?locale=). Defaults to 'en' for the
-- international positioning; existing rows adopt the default.
ALTER TABLE projects
    ADD COLUMN IF NOT EXISTS serp_locale text NOT NULL DEFAULT 'en';

-- All rows carry the non-blank default 'en', so the check validates immediately.
ALTER TABLE projects
    ADD CONSTRAINT projects_serp_locale_not_blank
        CHECK (length(btrim(serp_locale)) > 0);
