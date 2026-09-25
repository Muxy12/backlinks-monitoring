-- 0018 — suspected_lost status + granular change events (monitoring-engine §3/§4)
--
-- `suspected_lost` is the honest middle state: the page loaded fine but the link
-- was absent once — not yet confirmed lost (§24). Granular rel events replace the
-- coarse rel_changed for sponsored/ugc, and link_moved records an occurrence that
-- changed page region. Enum values only — ADD VALUE is additive and safe; the new
-- values are not used within this migration's transaction.

ALTER TYPE backlink_status ADD VALUE IF NOT EXISTS 'suspected_lost';

ALTER TYPE event_type ADD VALUE IF NOT EXISTS 'sponsored_added';
ALTER TYPE event_type ADD VALUE IF NOT EXISTS 'sponsored_removed';
ALTER TYPE event_type ADD VALUE IF NOT EXISTS 'ugc_added';
ALTER TYPE event_type ADD VALUE IF NOT EXISTS 'ugc_removed';
ALTER TYPE event_type ADD VALUE IF NOT EXISTS 'link_moved';
ALTER TYPE event_type ADD VALUE IF NOT EXISTS 'source_404_to_200';
