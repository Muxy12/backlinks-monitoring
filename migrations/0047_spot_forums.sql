-- 0047 — Spot Finder (Lot E): forums & communities.
--
-- Among the SERP results for the user's keywords, some are forums / communities
-- (phpBB, Discourse, Reddit, generic /forum/…). These are relevant places where
-- the audience already discusses the topic — candidates for genuine
-- participation / contextual links ("ninja linking"), surfaced separately from
-- the editorial/buyable spots. Detection is free: it reuses the SERP data from
-- the spot search, so this table is (re)populated by the same run and replaced
-- on each re-search. Tenant-scoped via the composite FK to projects.

CREATE TABLE IF NOT EXISTS spot_forums (
    id                uuid PRIMARY KEY,
    organization_id   uuid NOT NULL REFERENCES organizations ON DELETE CASCADE,
    project_id        uuid NOT NULL,
    url               text NOT NULL,     -- the forum/thread page that ranked
    domain            text NOT NULL,     -- its registrable host
    forum_kind        text NOT NULL,     -- phpbb | vbulletin | xenforo | discourse | reddit | generic
    example_keyword   text,              -- a keyword it ranked for
    best_position     int,               -- best organic rank seen
    created_at        timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT spot_forums_url_not_blank CHECK (length(btrim(url)) > 0),
    CONSTRAINT spot_forums_project_fk FOREIGN KEY (organization_id, project_id)
        REFERENCES projects (organization_id, id) ON DELETE CASCADE,
    CONSTRAINT spot_forums_uq UNIQUE (project_id, url)
);

CREATE INDEX IF NOT EXISTS spot_forums_project_idx
    ON spot_forums (project_id, best_position ASC NULLS LAST);
