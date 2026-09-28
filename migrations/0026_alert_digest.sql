-- 0026 — per-destination alert digest cadence.
--
-- 'immediate' (default) keeps the current behavior: one message per notification.
-- 'hourly' / 'daily' batch all pending notifications into a single summary message
-- sent at most once per window (the delivery tick checks last_delivered_at).

ALTER TABLE alert_destinations
    ADD COLUMN IF NOT EXISTS digest text NOT NULL DEFAULT 'immediate';

ALTER TABLE alert_destinations
    ADD CONSTRAINT alert_destinations_digest CHECK (digest IN ('immediate', 'hourly', 'daily'));
