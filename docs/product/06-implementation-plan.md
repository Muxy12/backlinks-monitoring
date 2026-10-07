# Implementation Plan — LinkPulz

Where the product is, and the sequence to public launch and beyond. The original engineering phasing
is [`../13-roadmap.md`](../13-roadmap.md); this plan is the current, launch-oriented view.

**Target:** public **FREE launch** with Post-Link Impact / GSC presented as "coming soon" (no Google
app verification needed for this launch).

---

## 1. Status — built & live

Foundation and all core product areas are implemented and merged to `main`:

- ✅ Auth, teams/roles, tenant isolation, billing (Stripe Checkout/Portal/webhook, plan catalog,
  10-day trial), admin backoffice.
- ✅ Projects, backlink inventory, CSV **+ `.xlsx`** import, classification, tags + auto-tagger,
  warranties.
- ✅ Monitoring engine: scheduler + Postgres job queue + crawler + state machine; `/internal/tick`
  cron **confirmed running in prod**; selective monitoring; first-check feedback.
- ✅ Alerts (Resend), notifications, daily snapshot, dashboard/overview attention engine.
- ✅ Discovery (seed-crawl + DataForSEO/Bing), competitors + gap analysis, Tiers.
- ✅ Link-building campaigns (BETA, env-gated).
- ✅ Post-Link Impact (BETA): GSC OAuth + Search Analytics + URL Inspection; all four layers built
  (gated to operator for public launch).
- ✅ **Anchor text distribution** (core classifier + endpoint + `/backlinks` panel).
- ✅ Onboarding (guided tour + checklist), logo, consent-gated GA4, SEO canonical fixes, marketing
  copy cleanup.

## 2. Launch checklist — remaining

### 🔴 Launch-day switches (code ready, flip at launch)
1. Remove the site-wide `X-Robots-Tag: noindex, nofollow` in `apps/web/middleware.ts` — until then
   nothing indexes.
2. Flip the homepage from waitlist mode to the full marketing site + open registration.
3. Set `NEXT_PUBLIC_APP_URL=https://linkpulz.com` on Vercel (belt-and-braces).

### 🟡 Before / around launch
- 6 product screenshots for the `/home` `.ph` image slots (**operator asset**).
- QA of the connected app end-to-end (needs operator login): dashboard, add + monitoring, alerts,
  campaigns, settings, onboarding render, **billing trial → paid E2E through real Stripe**,
  registration (Turnstile + verify email + trial start).
- Infra: move Render/Neon off the free tier (cold starts/sleep) — business decision.

See memory `launch-readiness` for the authoritative running checklist.

## 3. Sequence to public launch

```
1. Operator QA of the connected app + billing trial E2E   (blocking)
2. Add the 6 home screenshots                              (content)
3. Infra → paid tier                                       (business)
4. Launch day: flip 🔴 switches (noindex off, home live, env set)
5. Post-deploy: verify GA4 Realtime, sitemap/indexing, first real signups
```

## 4. After launch (candidate roadmap)

- **Open Post-Link Impact / GSC to everyone** after Google app verification: drop `&& is_platform_admin`
  in `me_routes` (`features.gsc`).
- **Anchor distribution v2:** optional per-project "target keyword" tag to enable a true exact-match %
  (today deliberately omitted — no keyword data stored).
- **Impact phase 2 extras:** ranking **per query**, server-log crawl signals.
- **Reporting:** revisit white-label / scheduled reports only if demanded (explicitly out of scope now).

## 5. Working constraints (affect every change)

- CI is the Rust gate; the local box can't build `db`/`api`/`worker` — rely on `cargo test -p
  backlio-core`, schema-check, web typecheck locally, then CI.
- SQL must be static literals; new domain logic goes in `crates/core` as pure, unit-tested functions.
- Migrations additive (`0041+`), applied to Neon via `migrate-production.yml`.
- Every new tenant query filters `organization_id` via `OrgScope`.
