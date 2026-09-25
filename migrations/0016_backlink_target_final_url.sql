-- 0016 — record where the target itself lands after its own redirects
--
-- When a link is found, the monitor now optionally fetches the target URL (via
-- the same SSRF-safe fetch path) to capture its final URL after redirects.
-- `backlinks.target_http_status` already exists (0005) but was never written;
-- it is now populated alongside this new `target_final_url`. Nullable and
-- backward-compatible. The per-check `backlink_checks.target_status` column
-- (also from 0005) is likewise now written.

ALTER TABLE backlinks
    ADD COLUMN IF NOT EXISTS target_final_url text;
