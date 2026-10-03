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
Checkout + Customer Portal + a signature-verified, idempotent webhook
(`billing_routes.rs`). Monthly price ids per plan via env `STRIPE_PRICE_<PLAN>`
(`STRIPE_PRICE_STARTER/PRO/BUSINESS/AGENCY`); **annual** via
`STRIPE_PRICE_<PLAN>_ANNUAL`. Checkout takes `interval` (`monthly`/`annual`). The
webhook persists plan, status, period, `cancel_at_period_end`, `canceled_at` and
`trial_end`, and handles `invoice.*` dunning with a `past_due` grace window
(see P4 below).

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
- **P4 — billing: DONE.** Checkout takes an `interval` (`monthly`/`annual`);
  annual uses `STRIPE_PRICE_<PLAN>_ANNUAL` (`stripe::price_for_plan_interval`),
  and `plan_from_price` reverse-maps both prices. The billing page has a
  monthly/annual toggle. `customer.subscription.created/updated` now upserts the
  full row **keyed by org** (robust to event ordering): plan, status,
  `current_period_start/end`, `cancel_at_period_end`, `canceled_at`, `trial_end`
  (COALESCE keeps stored values a later event omits). Dunning: `invoice.payment_
  failed` → `past_due` (the grace state — `is_access_active` still grants access);
  `invoice.payment_succeeded`/`paid` → back to `active` (only from `past_due`, so
  it never overrides a trial or cancellation). `customer.subscription.deleted`
  stamps `canceled_at`. Upgrade/downgrade is safe by construction: plan comes
  from the Stripe price via the webhook and limits derive from the plan spec, so
  a downgrade only blocks new adds (existing data untouched). No schema change —
  migration 0010 already had the columns.
- **P6 — customer UI: DONE.** Public `/pricing` page on the marketing shell (4
  plans mirroring `plans.rs`, pure-CSS monthly/annual toggle, Pro "Most popular",
  value strip, FAQ, AggregateOffer + FAQPage JSON-LD, in sitemap); homepage
  `#pricing` shows the 4 real plans + link to `/pricing`; nav/footer point to
  `/pricing`. Billing page shows a **usage panel** (backlinks/projects/seats vs
  plan limits, from `GET /v1/orgs/:id/billing/usage`) and a trial/access notice.
  A global **trial banner** in the app shell reads `GET /v1/orgs/:id/billing`
  (now returns `trial_ends_at` / `trial_days_left` / `access_active`) and shows
  "N days left" (firmer ≤ 3 days) or "choose a plan" when access is inactive.
  *Trial emails (started / T-3 / T-1 / expired) remain a follow-up.*
- **P7 — admin: DONE.** `GET /v1/admin/subscriptions` lists every org with a
  subscription — plan, status, per-customer MRR (catalog-priced), trial end,
  renewal date, pending cancel and canceled date (the columns P4 now writes) —
  rendered as a **Subscriptions** tab in `/admin`. `GET /v1/admin/billing` gained
  a **trial funnel** (trials active vs. expired, converted, conversion %) plus
  ARR/ARPA, shown in the admin Revenue section. The Customer-360 drawer now shows
  the workspace's subscription dates. Revenue is from the catalog; COGS stays N/A
  until a provider cost exists (the admin "not yet instrumented" list now holds
  only provider/usage cost).
- **P8 — analytics:** signup attribution + funnel events (needs a sink).

## Manual production steps (when the relevant phase ships)
- Stripe: create Products with a **monthly and an annual** Price for each of the
  four plans and set `STRIPE_PRICE_STARTER/PRO/BUSINESS/AGENCY` (monthly) plus
  `STRIPE_PRICE_STARTER_ANNUAL/PRO_ANNUAL/BUSINESS_ANNUAL/AGENCY_ANNUAL`, then
  `STRIPE_SECRET_KEY`, `STRIPE_WEBHOOK_SECRET`, `APP_URL`. Enable the webhook
  events `checkout.session.completed`, `customer.subscription.created/updated/
  deleted`, and `invoice.payment_failed`/`invoice.payment_succeeded`. A missing
  annual price makes only the annual checkout fail with a clear message; monthly
  is unaffected.
- DataForSEO is **not integrated**; its "cost protection / allowances" can only be
  generic provider-usage scaffolding until a provider exists (see docs/19).
