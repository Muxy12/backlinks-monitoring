-- 0034 link classification: four manual categorisation dimensions on a backlink.
-- acquisition (how the link was obtained), link_attribute (a manual override of
-- the crawler-detected Follow/Nofollow/Sponsored/UGC), placement (where on the
-- page), and link_format (the content format). All nullable = unclassified.
-- Values are validated in the API against a controlled vocabulary kept in code,
-- so the vocabulary can grow without another migration. When link_attribute is
-- NULL the effective attribute falls back to what the crawler observed
-- (detected_rel_attributes / detected_is_dofollow).

ALTER TABLE backlinks ADD COLUMN acquisition    text;
ALTER TABLE backlinks ADD COLUMN link_attribute text;
ALTER TABLE backlinks ADD COLUMN placement      text;
ALTER TABLE backlinks ADD COLUMN link_format    text;

-- Slicing the list by a single dimension, within an org (partial: only
-- classified rows). The attribute filter is a computed fallback expression, so
-- it is not indexed here.
CREATE INDEX backlinks_acquisition_idx ON backlinks (organization_id, acquisition) WHERE acquisition IS NOT NULL;
CREATE INDEX backlinks_placement_idx   ON backlinks (organization_id, placement)   WHERE placement IS NOT NULL;
CREATE INDEX backlinks_format_idx      ON backlinks (organization_id, link_format) WHERE link_format IS NOT NULL;
