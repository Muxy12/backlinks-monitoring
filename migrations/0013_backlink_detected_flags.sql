-- 0013 — detected follow/noindex flags on backlinks
-- The state machine (docs/08) compares the previous detection's dofollow state
-- and page noindex against the new observation. Storing them as booleans avoids
-- re-deriving from detected_rel_attributes / source_robots_meta on every check.
-- Additive and backward-compatible (ADR-0005): existing code ignores them.

ALTER TABLE backlinks
    ADD COLUMN detected_is_dofollow  boolean,
    ADD COLUMN source_robots_noindex boolean NOT NULL DEFAULT false;
