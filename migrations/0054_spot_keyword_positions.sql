-- Spot Finder: per-(spot, keyword) SERP positions — the ground truth behind the
-- thematic-performance scores (PTA/PTP, Référencime-style). One row per keyword
-- a spot actually ranks for; keywords it does NOT rank for are simply absent
-- (they still count against PTA via the project's total keyword set). Captured
-- by the SERP search run (find_spots); replaced with each run because the spots
-- themselves are replaced (ON DELETE CASCADE from spots).

CREATE TABLE IF NOT EXISTS spot_keyword_positions (
    id              uuid PRIMARY KEY,
    organization_id uuid NOT NULL REFERENCES organizations ON DELETE CASCADE,
    project_id      uuid NOT NULL,
    spot_id         uuid NOT NULL REFERENCES spots (id) ON DELETE CASCADE,
    keyword         text NOT NULL,
    position        int  NOT NULL,            -- best organic rank for this (spot, keyword), 1 = top
    page_url        text,                     -- the ranking page on the spot's domain
    search_volume   int,                      -- monthly searches, NULL when unknown
    checked_at      timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT skp_position_valid CHECK (position >= 1),
    CONSTRAINT skp_project_fk FOREIGN KEY (organization_id, project_id)
        REFERENCES projects (organization_id, id) ON DELETE CASCADE,
    CONSTRAINT skp_uq UNIQUE (spot_id, keyword)
);

CREATE INDEX IF NOT EXISTS skp_project_idx ON spot_keyword_positions (project_id);
CREATE INDEX IF NOT EXISTS skp_spot_idx ON spot_keyword_positions (spot_id);
