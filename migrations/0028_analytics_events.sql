-- 0028 — product analytics: funnel events + signup attribution (docs/20 §P8)
-- A self-hosted event sink (no external analytics provider is integrated). Each
-- row is one funnel event; first-touch marketing attribution is captured on the
-- 'signup' event. user_id / organization_id are nullable and SET NULL on delete
-- so historical funnel counts survive account/workspace deletion.

CREATE TABLE analytics_events (
    id               uuid        PRIMARY KEY,
    created_at       timestamptz NOT NULL DEFAULT now(),
    -- 'signup' | 'trial_started' | 'activated' | 'subscribed' (free-form text so
    -- new events need no migration; readers filter by the names they know).
    event            text        NOT NULL,
    user_id          uuid        REFERENCES users(id) ON DELETE SET NULL,
    organization_id  uuid        REFERENCES organizations(id) ON DELETE SET NULL,
    -- First-touch marketing attribution (populated on 'signup').
    utm_source       text,
    utm_medium       text,
    utm_campaign     text,
    utm_term         text,
    utm_content      text,
    referrer         text,
    landing_path     text,
    props            jsonb       NOT NULL DEFAULT '{}'::jsonb
);

-- Funnel counts over a time window, per event.
CREATE INDEX analytics_events_event_idx ON analytics_events (event, created_at DESC);
-- Attribution breakdown of signups by source.
CREATE INDEX analytics_events_source_idx
    ON analytics_events (utm_source) WHERE event = 'signup';
