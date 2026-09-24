-- 0007 — tags and backlink tagging
-- See docs/03-database-schema.md §3.8, §35 (tag filter)

CREATE TABLE tags (
    id              uuid PRIMARY KEY,
    organization_id uuid        NOT NULL REFERENCES organizations ON DELETE CASCADE,
    name            citext      NOT NULL,
    color           text,
    created_at      timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT tags_name_not_blank CHECK (length(btrim(name)) > 0),
    CONSTRAINT tags_color_format   CHECK (color IS NULL OR color ~ '^#[0-9a-fA-F]{6}$'),
    CONSTRAINT tags_org_name_uq    UNIQUE (organization_id, name),
    CONSTRAINT tags_org_id_uq      UNIQUE (organization_id, id)
);

CREATE TABLE backlink_tags (
    organization_id uuid        NOT NULL,
    backlink_id     uuid        NOT NULL,
    tag_id          uuid        NOT NULL,
    created_at      timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (backlink_id, tag_id),
    -- Both sides carry the composite FK, so a tag from org A cannot be
    -- attached to a backlink in org B (docs/03 §1.1).
    CONSTRAINT backlink_tags_backlink_fk FOREIGN KEY (organization_id, backlink_id)
        REFERENCES backlinks (organization_id, id) ON DELETE CASCADE,
    CONSTRAINT backlink_tags_tag_fk FOREIGN KEY (organization_id, tag_id)
        REFERENCES tags (organization_id, id) ON DELETE CASCADE
);

-- Filtering backlinks by tag (§35) walks this direction.
CREATE INDEX backlink_tags_tag_idx ON backlink_tags (tag_id);
