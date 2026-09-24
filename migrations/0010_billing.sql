-- 0010 — subscriptions, usage metering, Stripe webhook idempotency
-- See docs/03-database-schema.md §3.8, §39, docs/11-security-risks.md S5

CREATE TABLE subscriptions (
    id                     uuid PRIMARY KEY,
    organization_id        uuid              NOT NULL UNIQUE
                               REFERENCES organizations ON DELETE CASCADE,
    stripe_customer_id     text              NOT NULL,
    stripe_subscription_id text              UNIQUE,
    plan                   subscription_plan NOT NULL,
    status                 text              NOT NULL,
    current_period_start   timestamptz,
    current_period_end     timestamptz,
    cancel_at_period_end   boolean           NOT NULL DEFAULT false,
    canceled_at            timestamptz,
    trial_end              timestamptz,
    created_at             timestamptz       NOT NULL DEFAULT now(),
    updated_at             timestamptz       NOT NULL DEFAULT now(),
    -- Mirrors Stripe's subscription statuses. Entitlements are derived from
    -- this, never from request parameters (S5).
    CONSTRAINT subscriptions_status CHECK (status IN (
        'trialing', 'active', 'past_due', 'canceled',
        'incomplete', 'incomplete_expired', 'unpaid', 'paused'
    )),
    CONSTRAINT subscriptions_period
        CHECK (current_period_end IS NULL OR current_period_start IS NULL
               OR current_period_end >= current_period_start)
);

CREATE INDEX subscriptions_customer_idx ON subscriptions (stripe_customer_id);

CREATE TRIGGER subscriptions_set_updated_at
    BEFORE UPDATE ON subscriptions
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- THE webhook idempotency mechanism required by §39. An insert conflict means
-- "already handled, return 200". Replay and out-of-order delivery both become
-- no-ops rather than double effects (S5, docs/07 §4).
CREATE TABLE stripe_events (
    event_id     text PRIMARY KEY,
    event_type   text        NOT NULL,
    processed_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX stripe_events_processed_idx ON stripe_events (processed_at);

-- Per-org metering for plan limits (§39) and the crawler economics roll-up
-- (§53, docs/12 §4).
CREATE TABLE usage_records (
    id                  uuid PRIMARY KEY,
    organization_id     uuid        NOT NULL REFERENCES organizations ON DELETE CASCADE,
    period_start        date        NOT NULL,
    backlinks_monitored int         NOT NULL DEFAULT 0,
    checks_performed    int         NOT NULL DEFAULT 0,
    http_requests       int         NOT NULL DEFAULT 0,
    bytes_downloaded    bigint      NOT NULL DEFAULT 0,
    recorded_at         timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT usage_records_period_uq UNIQUE (organization_id, period_start),
    CONSTRAINT usage_records_non_negative CHECK (
        backlinks_monitored >= 0 AND checks_performed >= 0
        AND http_requests   >= 0 AND bytes_downloaded >= 0
    )
);

CREATE INDEX usage_records_org_idx ON usage_records (organization_id, period_start DESC);
