-- 0015 — capture link nature + loss date on backlinks
--
-- link_type / image_alt: the crawler now records whether the matched link wraps
-- an image (docs/06 §3.2) and that image's alt text. lost_at: the moment a link
-- transitioned into `lost`, materialized on write so it can be shown/sorted
-- without walking the event history. All nullable and backward-compatible, so
-- adding them does not disturb a running older binary.

ALTER TABLE backlinks
    ADD COLUMN IF NOT EXISTS link_type text,
    ADD COLUMN IF NOT EXISTS image_alt text,
    ADD COLUMN IF NOT EXISTS lost_at   timestamptz;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'backlinks_link_type_valid'
    ) THEN
        ALTER TABLE backlinks
            ADD CONSTRAINT backlinks_link_type_valid
            CHECK (link_type IS NULL OR link_type IN ('text', 'image'));
    END IF;
END $$;
