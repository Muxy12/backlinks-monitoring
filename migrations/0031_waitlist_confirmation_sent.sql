-- 0031 — track which waitlist signups have had their confirmation email sent.
-- NULL = not yet confirmed. Lets the admin "resend pending confirmations" action
-- be idempotent (only emails those who never got one) and lets the join handler
-- mark a signup once its confirmation actually sends.

ALTER TABLE waitlist_signups ADD COLUMN confirmation_sent_at timestamptz;
