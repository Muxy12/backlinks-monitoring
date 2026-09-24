-- 0011 — in-app notifications and per-user delivery preferences
-- See docs/03-database-schema.md §3.8, §37

CREATE TABLE notifications (
    id              uuid PRIMARY KEY,
    organization_id uuid NOT NULL REFERENCES organizations ON DELETE CASCADE,
    -- Null = an org-wide notification shown to every member.
    user_id         uuid REFERENCES users ON DELETE CASCADE,
    backlink_id     uuid,
    event_id        uuid REFERENCES backlink_events ON DELETE SET NULL,
    kind            text        NOT NULL,
    title           text        NOT NULL,
    body            text,
    data            jsonb,
    -- Delivery deduplication (docs/07 §4): (backlink_id, event_type, detected_at)
    -- hashed into a stable key so a re-run does not notify twice.
    dedupe_key      text,
    read_at         timestamptz,
    emailed_at      timestamptz,
    created_at      timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT notifications_title_not_blank CHECK (length(btrim(title)) > 0),
    CONSTRAINT notifications_backlink_fk FOREIGN KEY (organization_id, backlink_id)
        REFERENCES backlinks (organization_id, id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX notifications_dedupe_uq ON notifications (dedupe_key)
    WHERE dedupe_key IS NOT NULL;

-- The unread badge and the alerts feed.
CREATE INDEX notifications_unread_idx
    ON notifications (organization_id, created_at DESC)
    WHERE read_at IS NULL;
CREATE INDEX notifications_user_idx
    ON notifications (user_id, created_at DESC)
    WHERE user_id IS NOT NULL;
-- Pending email fan-out (§37).
CREATE INDEX notifications_pending_email_idx
    ON notifications (created_at)
    WHERE emailed_at IS NULL;

-- Per-user, per-event-type, per-channel (§37). The `channel` check is
-- deliberately wider than the MVP: slack/webhook/discord rows can be written
-- before their senders exist, because the channel trait is defined in Phase 4
-- and implemented later (docs/13, out of scope list).
CREATE TABLE notification_preferences (
    id              uuid PRIMARY KEY,
    user_id         uuid    NOT NULL REFERENCES users ON DELETE CASCADE,
    organization_id uuid    NOT NULL REFERENCES organizations ON DELETE CASCADE,
    event_type      event_type NOT NULL,
    channel         text    NOT NULL,
    enabled         boolean NOT NULL DEFAULT true,
    -- 'immediate' | 'hourly' | 'daily' — batching per §37.
    digest          text    NOT NULL DEFAULT 'immediate',
    CONSTRAINT notification_prefs_channel CHECK (channel IN ('in_app', 'email', 'slack', 'webhook', 'discord')),
    CONSTRAINT notification_prefs_digest  CHECK (digest IN ('immediate', 'hourly', 'daily')),
    CONSTRAINT notification_prefs_uq UNIQUE (user_id, organization_id, event_type, channel)
);

CREATE INDEX notification_prefs_lookup_idx
    ON notification_preferences (organization_id, event_type, channel)
    WHERE enabled;
