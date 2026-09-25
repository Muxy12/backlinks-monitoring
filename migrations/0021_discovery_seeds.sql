-- 0021 — discovery seeds: referring domains to expand-crawl (spec §21, §22)
--
-- Backlink discovery is a reverse-link problem, so a home crawler cannot find
-- unknown referring DOMAINS at web scale. But given a KNOWN referring domain
-- (e.g. imported from a GSC "Top linking sites" export), we can crawl that
-- domain to find the actual pages that link to us — turning domain-level hints
-- into real source_url -> target_url backlinks. These seeds are that queue.

CREATE TABLE IF NOT EXISTS discovery_seeds (
    id                uuid PRIMARY KEY,
    organization_id   uuid NOT NULL REFERENCES organizations ON DELETE CASCADE,
    project_id        uuid NOT NULL,
    domain            text NOT NULL,
    normalized_domain text NOT NULL,
    source            text NOT NULL DEFAULT 'import',   -- 'gsc' | 'import' | 'manual'
    status            text NOT NULL DEFAULT 'pending',  -- pending | crawling | done | error
    pages_crawled     int  NOT NULL DEFAULT 0,
    backlinks_found   int  NOT NULL DEFAULT 0,
    last_crawled_at   timestamptz,
    next_crawl_at     timestamptz NOT NULL DEFAULT now(),
    last_error        text,
    created_at        timestamptz NOT NULL DEFAULT now(),
    updated_at        timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT discovery_seeds_domain_not_blank CHECK (length(btrim(normalized_domain)) > 0),
    CONSTRAINT discovery_seeds_status_valid
        CHECK (status IN ('pending', 'crawling', 'done', 'error')),
    CONSTRAINT discovery_seeds_project_fk FOREIGN KEY (organization_id, project_id)
        REFERENCES projects (organization_id, id) ON DELETE CASCADE,
    CONSTRAINT discovery_seeds_uq UNIQUE (project_id, normalized_domain)
);

CREATE TRIGGER discovery_seeds_set_updated_at
    BEFORE UPDATE ON discovery_seeds
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- Due queue for the expansion crawler (small, only crawlable seeds).
CREATE INDEX IF NOT EXISTS discovery_seeds_due_idx
    ON discovery_seeds (next_crawl_at)
    WHERE status IN ('pending', 'done');
CREATE INDEX IF NOT EXISTS discovery_seeds_project_idx ON discovery_seeds (project_id);
