# 20 — Pricing, plans & trial

The public offer and how it is enforced. **One source of truth** for prices,
limits, monitoring interval and entitlements: `crates/billing/src/plans.rs`
(served to every UI via `GET /v1/plans`). No price or limit is hardcoded in the
frontend — change it in `plans.rs` and the pricing page, billing page, usage
displays and server enforcement all follow.

There is **no permanent free plan** — every plan has a **10-day free trial, no
credit card**, monitoring **every 48 hours**.

## The four plans

| Plan | Monthly | Annual (~2 mo free) | Backlinks | Projects | Seats |
|------|--------:|--------------------:|----------:|---------:|------:|
| Starter | $29 | $290 (≈$24/mo) | 500 | 3 | 1 |
| **Pro** (most popular) | $59 | $590 (≈$49/mo) | 2,500 | 10 | 3 |
| Business | $99 | $990 (≈$82/mo) | 5,000 | 25 | 5 |
| Agency | $199 | $1,990 (≈$166/mo) | 15,000 | 75 | 15 |

Seats include the owner. Over-limit **blocks new adds but never stops monitoring
or deletes data** (`entitlements.rs`, §39). Values live only in `plans.rs`.

## Source of truth & layering (§39)
- **Plan** → `billing::Plan` (enum, slugs `starter/pro/business/agency`; DB enum
  `subscription_plan`, migration 0027).
- **Limits** → `PlanSpec.{backlink,project,seat}_limit` (`None` = unlimited).
- **Entitlement checks** → `billing::entitlements` (`can_add_backlinks/project/seat`).
- **Catalog API** → `GET /v1/plans` (`billing_routes::list_plans`, public).

## Trial model (to build — P5)
On org creation, open a `trialing` subscription with `trial_end = now + 10 days`
(no Stripe customer). Trial entitlement = Starter limits (500 backlinks / 3
projects). On expiry: pause monitoring (scheduler skips the org), preserve all
data, show "choose a plan". Idempotent trial emails (started / T-3 / T-1 /
expired).

## Billing (Stripe)
Checkout + Customer Portal + a signature-verified, idempotent webhook already
exist (`billing_routes.rs`). Price ids per plan via env `STRIPE_PRICE_<PLAN>`
(`STRIPE_PRICE_STARTER/PRO/BUSINESS/AGENCY`). **Annual** interval and the
period/cancel/trial webhook columns are P4 (not yet built).

## Implementation status
- **P2 — central config / source of truth: DONE.** `plans.rs` re-spec (4 plans +
  annual + 48h + most-popular), enum migration 0027 (`agency_plus → business`
  re-spec, pre-launch remap by price), `GET /v1/plans`, UI mirrors updated.
- **P3 — server-side enforcement: DONE.** `can_add_project` wired into
  create_project; `can_add_seat` into invite + accept (owner counts as a seat).
  Tier-2 is intentionally out of the backlink cap (`count_active` is tier-1 only).
  Bulk-import "import what fits" partial behavior is still a follow-up.
- **P5 — trial + cost-gating: core DONE.** A 10-day no-card `trialing` subscription
  is opened on org creation (`billing::start_trial`, Starter entitlement). The
  scheduler (`enqueue_due`) now only monitors orgs that are not suspended and have
  an active/past_due subscription or an un-expired trial. New backlink/project/seat
  adds are blocked once the trial expires or the subscription is inactive
  (`billing::is_access_active`), without touching existing data. *Trial UX banner +
  trial emails (started / T-3 / T-1 / expired) are P6/follow-up.*
- **P4 — billing:** annual prices/interval, write period/cancel/trial columns,
  handle `invoice.*` + past-due grace, safe upgrade/downgrade.
- **P6 — customer UI:** real `/pricing` page (monthly/annual toggle, FAQ), billing
  usage, upgrade prompts, trial banner.
- **P7 — admin:** subscriptions table, trial funnel, per-customer economics
  (revenue from the catalog; COGS N/A until provider cost exists).
- **P8 — analytics:** signup attribution + funnel events (needs a sink).

## Manual production steps (when the relevant phase ships)
- Stripe: create Products/Prices for the four plans (monthly; annual in P4) and
  set `STRIPE_PRICE_STARTER/PRO/BUSINESS/AGENCY` (+ `_ANNUAL` in P4),
  `STRIPE_SECRET_KEY`, `STRIPE_WEBHOOK_SECRET`, `APP_URL`.
- DataForSEO is **not integrated**; its "cost protection / allowances" can only be
  generic provider-usage scaffolding until a provider exists (see docs/19).
