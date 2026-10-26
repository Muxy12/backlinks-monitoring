-- Source page's resolved IP at the last check (the final hop's address, the same
-- one the SSRF guard already validates). Stored as text; shown in the backlinks
-- list and a building block for a later shared-IP / same-C-class network
-- footprint signal. Preserved across failed checks (COALESCE on write).
ALTER TABLE backlinks ADD COLUMN source_ip text;
