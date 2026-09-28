-- 0024 — multi-channel alert destinations + per-destination delivery ledger.
--
-- notification_preferences (0011) already models per-user, per-event, per-channel
-- opt-in. What was missing is WHERE to deliver org-level channels: a Slack
-- Incoming Webhook URL or a generic webhook endpoint. These are org-scoped
-- destinations. Outbound POSTs to these user-supplied URLs go through the same
-- SSRF guard as the crawler (scheme/host/port/IP + pinned connection).
--
-- alert_deliveries is the idempotency ledger: one row per (destination,
-- notification) so a re-run of the delivery tick never double-sends.

CREATE TABLE alert_destinations (
    id                uuid PRIMARY KEY,
    organization_id   uuid NOT NULL REFERENCES organizations ON DELETE CASCADE,
    kind              text NOT NULL,                 -- 'webhook' | 'slack'
    target            text NOT NULL,                 -- the destination URL
    name              text NOT NULL DEFAULT '',
    enabled           boolean NOT NULL DEFAULT true,
    last_status       text,                          -- 'ok' | 'error' | NULL (never used)
    last_error        text,
    last_delivered_at timestamptz,
    created_at        timestamptz NOT NULL DEFAULT now(),
    updated_at        timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT alert_destinations_kind CHECK (kind IN ('webhook', 'slack')),
    CONSTRAINT alert_destinations_target_not_blank CHECK (length(btrim(target)) > 0)
);

CREATE INDEX alert_destinations_org_idx ON alert_destinations (organization_id);
CREATE INDEX alert_destinations_active_idx ON alert_destinations (id) WHERE enabled;

CREATE TRIGGER alert_destinations_set_updated_at
    BEFORE UPDATE ON alert_destinations
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TABLE alert_deliveries (
    id              uuid PRIMARY KEY,
    destination_id  uuid NOT NULL REFERENCES alert_destinations ON DELETE CASCADE,
    notification_id uuid NOT NULL REFERENCES notifications ON DELETE CASCADE,
    status          text NOT NULL,                   -- 'ok' | 'error'
    http_status     int,
    error           text,
    created_at      timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT alert_deliveries_status CHECK (status IN ('ok', 'error')),
    CONSTRAINT alert_deliveries_uq UNIQUE (destination_id, notification_id)
);

CREATE INDEX alert_deliveries_notif_idx ON alert_deliveries (notification_id);
