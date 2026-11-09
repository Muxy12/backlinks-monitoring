-- Source page's resolved IP at the last check (the final hop's address, the same
-- one the SSRF guard already validates). Stored as text; shown in the backlinks
-- list and a building block for a later shared-IP / same-C-class network
-- footprint signal. Preserved across failed checks (COALESCE on write).
--
-- IDEMPOTENT: `source_ip` was hand-added on prod before its _sqlx_migrations row
-- was recorded, so guard the column so `sqlx migrate run` records version 48 and
-- continues instead of aborting on an "already exists".
ALTER TABLE backlinks ADD COLUMN IF NOT EXISTS source_ip text;
