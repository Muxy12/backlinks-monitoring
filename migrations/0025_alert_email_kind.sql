-- 0025 — allow 'email' as an alert destination kind (Resend).
--
-- Email joins webhook + slack as a first-class alert channel: the same
-- destinations + deliveries plumbing (0024) drives it, so it gets the delivery
-- ledger, the Test button and per-destination status for free. `target` holds an
-- email address instead of a URL. Sending goes through Resend's HTTP API, gated
-- on RESEND_API_KEY / RESEND_FROM (absent -> the destination reports "email not
-- configured" rather than sending).

ALTER TABLE alert_destinations DROP CONSTRAINT alert_destinations_kind;
ALTER TABLE alert_destinations
    ADD CONSTRAINT alert_destinations_kind CHECK (kind IN ('webhook', 'slack', 'email'));
