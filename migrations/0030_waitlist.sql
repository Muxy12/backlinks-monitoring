-- 0030 — pre-launch waitlist (email capture on the "coming soon" homepage)
-- Public, unauthenticated signups. Email is stored normalized (lowercased,
-- trimmed by the handler) and UNIQUE so a repeat submit is a no-op. First-touch
-- marketing attribution is captured alongside, like signup (docs/20 §P8).

CREATE TABLE waitlist_signups (
    id            uuid        PRIMARY KEY,
    email         text        NOT NULL UNIQUE,
    created_at    timestamptz NOT NULL DEFAULT now(),
    utm_source    text,
    utm_medium    text,
    utm_campaign  text,
    referrer      text,
    landing_path  text,
    CONSTRAINT waitlist_email_shape
        CHECK (char_length(email) BETWEEN 3 AND 320 AND position('@' in email) > 1)
);

CREATE INDEX waitlist_signups_created_idx ON waitlist_signups (created_at DESC);
