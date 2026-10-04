-- 0032 — per-signup unsubscribe token for the waitlist confirmation email.
-- Opaque random handle so the unsubscribe link carries no email in the URL.
-- New signups get an app-generated token; existing rows are backfilled.

ALTER TABLE waitlist_signups ADD COLUMN unsubscribe_token text;

UPDATE waitlist_signups
   SET unsubscribe_token = md5(random()::text || id::text || clock_timestamp()::text)
 WHERE unsubscribe_token IS NULL;

CREATE UNIQUE INDEX waitlist_unsubscribe_token_idx
    ON waitlist_signups (unsubscribe_token);
