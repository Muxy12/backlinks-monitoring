-- 0046 — Spot Finder (Lot D): cross-reference spots with link-buying platforms.
--
-- Each spot (a domain that ranks for the user's keywords) is looked up in the
-- configured marketplace catalogues (LinkAvista today; the provider layer is
-- pluggable, so more can be added). A match marks the spot as buyable and
-- records the price + the marketplace listing, so the user sees "you can buy a
-- link here, for N" next to the relevance/quality signals. Columns live on the
-- spot row (NULL = not matched / not listed) and reset on a SERP re-run.
-- `spot_marketplace_runs` tracks one matching pass.

ALTER TABLE spots
    ADD COLUMN IF NOT EXISTS marketplace         text,    -- e.g. 'linkavista'
    ADD COLUMN IF NOT EXISTS marketplace_site_id bigint,  -- the listing's id on that platform
    ADD COLUMN IF NOT EXISTS buy_price_cents      bigint,  -- placement price, platform's money unit
    ADD COLUMN IF NOT EXISTS buy_currency         text,
    ADD COLUMN IF NOT EXISTS marketplace_url      text,
    ADD COLUMN IF NOT EXISTS matched_at           timestamptz;

CREATE TABLE IF NOT EXISTS spot_marketplace_runs (
    id                uuid PRIMARY KEY,
    organization_id   uuid NOT NULL REFERENCES organizations ON DELETE CASCADE,
    project_id        uuid NOT NULL,
    status            text NOT NULL DEFAULT 'queued',  -- queued | running | done | error
    spots_checked     int  NOT NULL DEFAULT 0,
    matches_found     int  NOT NULL DEFAULT 0,
    provider_queries  int  NOT NULL DEFAULT 0,
    error             text,
    created_at        timestamptz NOT NULL DEFAULT now(),
    updated_at        timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT spot_marketplace_runs_status_valid
        CHECK (status IN ('queued', 'running', 'done', 'error')),
    CONSTRAINT spot_marketplace_runs_project_fk FOREIGN KEY (organization_id, project_id)
        REFERENCES projects (organization_id, id) ON DELETE CASCADE
);

CREATE TRIGGER spot_marketplace_runs_set_updated_at
    BEFORE UPDATE ON spot_marketplace_runs
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE INDEX IF NOT EXISTS spot_marketplace_runs_project_idx
    ON spot_marketplace_runs (project_id, created_at DESC);
