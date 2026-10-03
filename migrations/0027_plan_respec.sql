-- 0027 — pricing overhaul: re-spec the plan catalog.
--
-- The public offer becomes Starter / Pro / Business / Agency (docs/20). The old
-- `agency_plus` tier is retired; prices and limits are redefined in the single
-- source of truth `crates/billing/src/plans.rs`.
--
-- Pre-launch: there are no real paying subscriptions. Any existing rows are
-- remapped by the PRICE the customer pays, so nobody's billing changes meaning:
--   agency      ($99)  -> business  ($99)
--   agency_plus ($199) -> agency    ($199)
--   starter / pro keep their slug.
-- Postgres can't rename enum values in place, so we swap the type. All DDL is in
-- one transaction (sqlx wraps each migration); subscriptions.plan is the only
-- column using the type, and it has no default.

ALTER TYPE subscription_plan RENAME TO subscription_plan_old;

CREATE TYPE subscription_plan AS ENUM ('starter', 'pro', 'business', 'agency');

ALTER TABLE subscriptions
    ALTER COLUMN plan TYPE subscription_plan
    USING (
        CASE plan::text
            WHEN 'agency_plus' THEN 'agency'
            WHEN 'agency'      THEN 'business'
            ELSE plan::text
        END
    )::subscription_plan;

DROP TYPE subscription_plan_old;
