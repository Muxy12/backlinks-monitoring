-- 0036 link-building campaigns (BETA). Turns a campaign from a buy-tracking
-- record into a goal-driven workflow: target pages to build links to, and a
-- prospect pipeline fed from the competitor-gap analysis. Purely additive — no
-- data backfill, so it is safe to hand-apply. Vocabularies (prospect stage /
-- source) are validated in the API and kept in code (like 0034/0035); only a
-- small, stable CHECK on stage is cheap insurance.

-- A campaign gains an objective and one headline target page (the full list of
-- target pages lives in campaign_targets).
ALTER TABLE campaigns
    ADD COLUMN objective          text,
    ADD COLUMN primary_target_url text;

-- Our own pages a campaign is building links to.
CREATE TABLE campaign_targets (
    id              uuid PRIMARY KEY,
    organization_id uuid        NOT NULL REFERENCES organizations ON DELETE CASCADE,
    campaign_id     uuid        NOT NULL,
    url             text        NOT NULL,
    label           text,
    created_at      timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT campaign_targets_url_not_blank CHECK (length(btrim(url)) > 0),
    CONSTRAINT campaign_targets_campaign_fk
        FOREIGN KEY (organization_id, campaign_id)
        REFERENCES campaigns (organization_id, id) ON DELETE CASCADE,
    CONSTRAINT campaign_targets_uq UNIQUE (campaign_id, url)
);
CREATE INDEX campaign_targets_campaign_idx ON campaign_targets (campaign_id);

-- The prospect pipeline: candidate sites to pursue for a campaign. `stage`
-- drives the kanban; `source` records where the prospect came from;
-- `score`/`score_reason` are the explainable priority (crates/core/src/scoring.rs);
-- `won_backlink_id` links a converted prospect to the backlink it became.
CREATE TABLE prospects (
    id                uuid          NOT NULL,
    organization_id   uuid          NOT NULL REFERENCES organizations ON DELETE CASCADE,
    campaign_id       uuid          NOT NULL,
    domain            text          NOT NULL,
    normalized_domain text          NOT NULL,
    stage             text          NOT NULL DEFAULT 'identified'
        CHECK (stage IN ('identified', 'qualified', 'contacted',
                         'negotiating', 'won', 'lost')),
    source            text          NOT NULL DEFAULT 'manual',
    score             int,
    score_reason      text,
    competitor_count  int,
    contact_name      text,
    contact_email     citext,
    target_url        text,
    estimated_cost    numeric(14, 2),
    currency          char(3),
    notes             text,
    last_contacted_at timestamptz,
    won_backlink_id   uuid,
    created_at        timestamptz   NOT NULL DEFAULT now(),
    updated_at        timestamptz   NOT NULL DEFAULT now(),
    PRIMARY KEY (id),
    CONSTRAINT prospects_domain_not_blank CHECK (length(btrim(domain)) > 0),
    CONSTRAINT prospects_currency_format CHECK (currency IS NULL OR currency ~ '^[A-Z]{3}$'),
    CONSTRAINT prospects_cost_pos CHECK (estimated_cost IS NULL OR estimated_cost >= 0),
    -- tenant-safe uniqueness per campaign (org travels in the FK).
    CONSTRAINT prospects_org_id_uq UNIQUE (organization_id, id),
    CONSTRAINT prospects_campaign_fk
        FOREIGN KEY (organization_id, campaign_id)
        REFERENCES campaigns (organization_id, id) ON DELETE CASCADE,
    CONSTRAINT prospects_campaign_domain_uq UNIQUE (campaign_id, normalized_domain)
);
-- Pipeline listing: a campaign's prospects filtered by stage.
CREATE INDEX prospects_campaign_stage_idx ON prospects (organization_id, campaign_id, stage);
-- Priority ordering within a campaign.
CREATE INDEX prospects_score_idx ON prospects (campaign_id, score DESC);
