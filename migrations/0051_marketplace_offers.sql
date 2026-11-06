-- 0051 — AI-era refactor, Phase 1: normalised multi-source marketplace offers.
--
-- See docs/product/21 §4.2. Today a spot carries at most ONE embedded offer
-- (columns on `spots`, first marketplace wins — 0046). Price-comparison needs
-- one spot → MANY offers, one per source. This adds the normalised tables; the
-- embedded `spots.marketplace*` columns are kept as a denormalised "cheapest
-- offer" cache so existing consumers (SpotView.buyable, spot→campaign, CSV)
-- keep working unchanged. Additive; nothing in the monitoring layer is touched.

-- The marketplaces we can price against. A source = one adapter impl of
-- `MarketplaceProvider`. Seeded with LinkAvista (the first, FR); international
-- sources are added as rows + an adapter.
CREATE TABLE IF NOT EXISTS marketplace_sources (
    id           uuid PRIMARY KEY,
    slug         text NOT NULL,            -- stable id, matches provider.name() e.g. 'linkavista'
    display_name text NOT NULL,
    locale       text,                     -- primary market, e.g. 'fr' | 'en'
    enabled      boolean NOT NULL DEFAULT true,
    created_at   timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT marketplace_sources_slug_uq UNIQUE (slug)
);

INSERT INTO marketplace_sources (id, slug, display_name, locale)
VALUES ('00000000-0000-0000-0000-0000000a4157', 'linkavista', 'LinkAvista', 'fr')
ON CONFLICT (slug) DO NOTHING;

-- One offer per (spot, source): the price-compare rows. Tenant-scoped via the
-- composite project FK; cascades when the spot is replaced on a SERP re-run, so
-- offers never outlive the spot they describe.
CREATE TABLE IF NOT EXISTS marketplace_offers (
    id                   uuid PRIMARY KEY,
    organization_id      uuid NOT NULL REFERENCES organizations ON DELETE CASCADE,
    project_id           uuid NOT NULL,
    spot_id              uuid NOT NULL REFERENCES spots (id) ON DELETE CASCADE,
    source_slug          text NOT NULL,        -- → marketplace_sources.slug
    editor_domain        text NOT NULL,
    site_id              bigint,               -- the listing id on that platform
    price_cents          bigint,               -- placement price, source money unit
    currency             text,
    dofollow             boolean,              -- NULL = source didn't say
    content_included     boolean,
    indexation_guarantee boolean,
    retention_months     int,
    offer_url            text,
    imported_at          timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT marketplace_offers_uq UNIQUE (spot_id, source_slug),
    CONSTRAINT marketplace_offers_project_fk FOREIGN KEY (organization_id, project_id)
        REFERENCES projects (organization_id, id) ON DELETE CASCADE
);

-- Price-compare reads all offers for a project's spots, cheapest first.
CREATE INDEX IF NOT EXISTS marketplace_offers_project_idx
    ON marketplace_offers (organization_id, project_id, spot_id);
