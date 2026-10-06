-- Post-Link Impact — Phase 2: Google Search Console URL Inspection results, one
-- latest-snapshot row per inspected target page. Feeds the Crawl + Indexation
-- layers of the impact view (coverageState, lastCrawlTime, canonical, verdict).
-- Populated by a bounded, quota-aware sweep (URL Inspection API: 2000/day per
-- property), only for pages of tracked backlinks. Observation, never causation.

CREATE TABLE gsc_url_inspection (
    organization_id     uuid        NOT NULL,
    project_id          uuid        NOT NULL,
    page_url_normalized text        NOT NULL,
    inspected_at        timestamptz NOT NULL DEFAULT now(),
    -- indexStatusResult fields (all nullable — Google may omit any of them).
    coverage_state      text,
    indexing_state      text,
    robots_txt_state    text,
    page_fetch_state    text,
    verdict             text,
    last_crawl_time     timestamptz,
    google_canonical    text,
    user_canonical      text,
    raw                 jsonb,
    PRIMARY KEY (organization_id, project_id, page_url_normalized)
);
