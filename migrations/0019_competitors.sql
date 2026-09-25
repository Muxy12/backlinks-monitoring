-- 0019 — competitor monitoring (backlink-intelligence spec §1–§3)
--
-- A competitor's backlink has the exact same shape as ours (source -> target),
-- so we reuse the backlinks table rather than a parallel one: tag each row with
-- `subject` (own | competitor) and, for competitors, which competitor it belongs
-- to. All main views default to subject='own' so competitor rows never pollute
-- the customer's own dashboard or plan cap.

CREATE TABLE IF NOT EXISTS project_competitors (
    id                uuid PRIMARY KEY,
    organization_id   uuid NOT NULL REFERENCES organizations ON DELETE CASCADE,
    project_id        uuid NOT NULL,
    domain            text NOT NULL,
    normalized_domain text NOT NULL,
    enabled           boolean NOT NULL DEFAULT true,
    first_scanned_at  timestamptz,
    last_scanned_at   timestamptz,
    created_at        timestamptz NOT NULL DEFAULT now(),
    updated_at        timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT project_competitors_domain_not_blank CHECK (length(btrim(normalized_domain)) > 0),
    CONSTRAINT project_competitors_project_fk FOREIGN KEY (organization_id, project_id)
        REFERENCES projects (organization_id, id) ON DELETE CASCADE,
    -- Needed as an FK target for backlinks.competitor_id (tenant-scoped).
    CONSTRAINT project_competitors_org_id_uq UNIQUE (organization_id, id),
    -- No duplicate competitor domain within a project.
    CONSTRAINT project_competitors_uq UNIQUE (project_id, normalized_domain)
);

CREATE TRIGGER project_competitors_set_updated_at
    BEFORE UPDATE ON project_competitors
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE INDEX project_competitors_project_idx ON project_competitors (project_id);

-- Tag backlinks with their subject + (for competitors) the competitor.
ALTER TABLE backlinks
    ADD COLUMN IF NOT EXISTS subject       text NOT NULL DEFAULT 'own',
    ADD COLUMN IF NOT EXISTS competitor_id uuid;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'backlinks_subject_valid') THEN
        ALTER TABLE backlinks ADD CONSTRAINT backlinks_subject_valid
            CHECK (subject IN ('own', 'competitor'));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'backlinks_competitor_fk') THEN
        ALTER TABLE backlinks ADD CONSTRAINT backlinks_competitor_fk
            FOREIGN KEY (organization_id, competitor_id)
            REFERENCES project_competitors (organization_id, id) ON DELETE CASCADE;
    END IF;
END $$;

-- Competitor backlinks are queried by (org, competitor); own views filter subject.
CREATE INDEX IF NOT EXISTS backlinks_competitor_idx
    ON backlinks (competitor_id) WHERE competitor_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS backlinks_org_subject_idx
    ON backlinks (organization_id, subject);

-- Competitor alert events (dedup handled at emission).
ALTER TYPE event_type ADD VALUE IF NOT EXISTS 'competitor_new_backlink';
ALTER TYPE event_type ADD VALUE IF NOT EXISTS 'competitor_new_referring_page';
ALTER TYPE event_type ADD VALUE IF NOT EXISTS 'competitor_new_referring_domain';
ALTER TYPE event_type ADD VALUE IF NOT EXISTS 'competitor_backlink_lost';
ALTER TYPE event_type ADD VALUE IF NOT EXISTS 'competitor_backlink_recovered';
