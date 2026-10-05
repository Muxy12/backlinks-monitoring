-- 0038 Google Search Console integration (Post-Link Impact BETA foundation).
-- One OAuth connection per project (project = one domain = one GSC property), and
-- a per-target-URL daily time-series of Search Analytics metrics. Both are the
-- data foundation for observing what happens to a target page after a backlink is
-- acquired. Tokens are stored encrypted (AES-256-GCM, app layer); the DB only
-- ever sees ciphertext. Purely additive.

CREATE TABLE gsc_connections (
    id                 uuid        PRIMARY KEY,
    organization_id    uuid        NOT NULL REFERENCES organizations ON DELETE CASCADE,
    project_id         uuid        NOT NULL,
    google_email       text,
    -- The mapped GSC property, e.g. 'sc-domain:example.com' or 'https://example.com/'.
    -- NULL until the user picks a property after consent.
    site_url           text,
    refresh_token_enc  bytea       NOT NULL,
    access_token_enc   bytea,
    access_expires_at  timestamptz,
    scopes             text,
    last_sync_at       timestamptz,
    last_error         text,
    created_at         timestamptz NOT NULL DEFAULT now(),
    updated_at         timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT gsc_connections_project_uq UNIQUE (project_id),
    CONSTRAINT gsc_connections_project_fk
        FOREIGN KEY (organization_id, project_id)
        REFERENCES projects (organization_id, id) ON DELETE CASCADE
);
CREATE INDEX gsc_connections_org_idx ON gsc_connections (organization_id);

-- Per-target-page daily Search Analytics (dimensions [date, page]). Deduplicated
-- by (project, page, day) so N backlinks to the same target reuse one row — the
-- Post-Link Impact service reads this locally, never calling Google on open.
CREATE TABLE gsc_daily (
    organization_id       uuid        NOT NULL REFERENCES organizations ON DELETE CASCADE,
    project_id            uuid        NOT NULL,
    page_url_normalized   text        NOT NULL,
    day                   date        NOT NULL,
    impressions           int         NOT NULL DEFAULT 0,
    clicks                int         NOT NULL DEFAULT 0,
    ctr                   float8      NOT NULL DEFAULT 0,
    position              float8      NOT NULL DEFAULT 0,
    fetched_at            timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (organization_id, project_id, page_url_normalized, day),
    CONSTRAINT gsc_daily_project_fk
        FOREIGN KEY (organization_id, project_id)
        REFERENCES projects (organization_id, id) ON DELETE CASCADE
);
-- Read path: one target page's series within a date range (impact windows).
CREATE INDEX gsc_daily_page_idx
    ON gsc_daily (organization_id, project_id, page_url_normalized, day);
