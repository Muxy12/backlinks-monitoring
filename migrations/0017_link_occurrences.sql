-- 0017 — physical link occurrences (docs/06 §3.4, monitoring-engine spec §1)
--
-- A backlink is one logical relation (source_url -> target_url). A single source
-- page can contain that link several times, in different places, with different
-- anchors. `backlinks` keeps the single "best" observation for fast dashboards;
-- this table records EVERY occurrence so we can show them and detect movement.
-- Reuses the backlink as the parent — it does not change the backlink model.

CREATE TABLE IF NOT EXISTS link_occurrences (
    id              uuid PRIMARY KEY,
    organization_id uuid NOT NULL,
    backlink_id     uuid NOT NULL,

    -- Stable-ish identity of one occurrence across crawls (normalized href +
    -- link type + anchor, disambiguated for exact duplicates). Lets a re-crawl
    -- recognize the same occurrence instead of churning rows.
    fingerprint  text NOT NULL,

    anchor_text  text,
    link_type    text,               -- 'text' | 'image'
    image_alt    text,
    href_raw     text,               -- exact href as written on the page
    rel_raw      text[],             -- raw rel tokens
    is_nofollow  boolean NOT NULL DEFAULT false,
    is_sponsored boolean NOT NULL DEFAULT false,
    is_ugc       boolean NOT NULL DEFAULT false,
    placement    text NOT NULL DEFAULT 'unknown',
    surrounding_text text,
    position     int,

    first_seen_at timestamptz NOT NULL DEFAULT now(),
    last_seen_at  timestamptz NOT NULL DEFAULT now(),
    removed_at    timestamptz,       -- NULL = still present on the page
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT link_occurrences_placement_valid CHECK (placement IN (
        'content','header','footer','sidebar','navigation','author_bio','comment','unknown')),
    CONSTRAINT link_occurrences_link_type_valid CHECK (
        link_type IS NULL OR link_type IN ('text','image')),
    CONSTRAINT link_occurrences_backlink_fk FOREIGN KEY (organization_id, backlink_id)
        REFERENCES backlinks (organization_id, id) ON DELETE CASCADE,
    CONSTRAINT link_occurrences_fingerprint_uq UNIQUE (backlink_id, fingerprint)
);

CREATE TRIGGER link_occurrences_set_updated_at
    BEFORE UPDATE ON link_occurrences
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- Drawer: the live occurrences of one backlink (most current first).
CREATE INDEX link_occurrences_backlink_idx
    ON link_occurrences (backlink_id, last_seen_at DESC)
    WHERE removed_at IS NULL;
-- Org-scoped counts / analytics.
CREATE INDEX link_occurrences_org_idx ON link_occurrences (organization_id);
