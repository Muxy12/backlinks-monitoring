-- Purchase campaigns (refonte P3, §8): a named basket of spots the user intends
-- to buy for a site — budget, selected spots (snapshotted), target pages, anchors
-- and per-spot order state. Distinct from the outreach "campaigns" (link-building
-- emailing); this one is "pick spots → buy → monitor".

CREATE TABLE IF NOT EXISTS purchase_campaigns (
    id              uuid PRIMARY KEY,
    organization_id uuid NOT NULL REFERENCES organizations ON DELETE CASCADE,
    project_id      uuid NOT NULL,
    name            text NOT NULL,
    status          text NOT NULL DEFAULT 'draft',   -- draft|ready|ordering|published|done
    budget_cents    bigint,
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT purchase_campaigns_name_not_blank CHECK (length(btrim(name)) > 0),
    CONSTRAINT purchase_campaigns_status_valid
        CHECK (status IN ('draft', 'ready', 'ordering', 'published', 'done')),
    CONSTRAINT purchase_campaigns_project_fk FOREIGN KEY (organization_id, project_id)
        REFERENCES projects (organization_id, id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS purchase_campaigns_project_idx ON purchase_campaigns (project_id);

CREATE TABLE IF NOT EXISTS purchase_campaign_items (
    id              uuid PRIMARY KEY,
    organization_id uuid NOT NULL REFERENCES organizations ON DELETE CASCADE,
    campaign_id     uuid NOT NULL REFERENCES purchase_campaigns ON DELETE CASCADE,
    spot_id         uuid REFERENCES spots (id) ON DELETE SET NULL,   -- snapshot keeps the line if the spot is later removed
    domain          text NOT NULL,
    price_cents     bigint,
    currency        text,
    source_slug     text,                              -- chosen marketplace
    target_url      text,                              -- the page the backlink should point to
    anchor          text,                              -- planned anchor text
    status          text NOT NULL DEFAULT 'todo',      -- todo|ordered|published|problem
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT purchase_campaign_items_status_valid
        CHECK (status IN ('todo', 'ordered', 'published', 'problem')),
    CONSTRAINT purchase_campaign_items_uq UNIQUE (campaign_id, spot_id)
);

CREATE INDEX IF NOT EXISTS purchase_campaign_items_campaign_idx
    ON purchase_campaign_items (campaign_id);
