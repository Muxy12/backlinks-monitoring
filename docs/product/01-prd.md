# Product Requirement Document — LinkPulz

> **One line:** LinkPulz tells SEO teams the moment a backlink they care about changes or disappears —
> and, honestly, what happened around it — without ever handing them an opaque "toxic score".

Status: **pre-public-launch** (public FREE launch planned; Post-Link Impact / Google Search Console
gated to the operator as "coming soon"). See [`06-implementation-plan.md`](06-implementation-plan.md)
and the engineering roadmap [`../13-roadmap.md`](../13-roadmap.md).

---

## 1. Problem

Backlinks are expensive to earn and easy to lose: a publisher edits a post, a page 404s, a `rel`
changes to `nofollow`, an anchor is rewritten. Most teams discover this weeks later, by accident. The
existing tools either (a) only re-crawl slowly and conflate a failed fetch with a genuine loss, or
(b) bury everything under a proprietary "toxicity" number nobody can audit.

## 2. Who it is for

| Persona | Need |
|---|---|
| **In-house SEO / growth** | Know immediately when an earned or paid link breaks; prove link value over time. |
| **SEO agencies** | Monitor many client sites from one workspace, with per-client separation and exportable change history. |
| **Link builders** | Run outreach campaigns, convert won prospects into monitored, protected backlinks, and track the acquired profile. |

## 3. Value proposition & non-negotiable principles

1. **A failed request is never a lost backlink.** Loss is only ever concluded from a positive
   *Absent* observation, encoded as a type in the state machine ([`../08-backlink-state-machine.md`](../08-backlink-state-machine.md)),
   not a heuristic. This is the product's core promise.
2. **Observation over verdict.** Everywhere — anchor distribution, Post-Link Impact — LinkPulz shows
   *what it observed* (before/after, counts, timelines) and never claims causation or a single
   composite score. The only "warnings" are explicitly framed as observations with a link to
   Google's own guidance.
3. **Honest marketing = product truth.** Copy only claims what the engine does. Frequency is "on a
   schedule you set"; reporting is honest CSV export + change history; no fabricated metrics.

## 4. Feature inventory (what it does)

### 4.1 Monitoring & backlink lifecycle
- Scheduled checks of each monitored backlink on a configurable cadence (per project / per backlink).
- State machine: `live → changed / redirected / nofollow / broken / lost → restored`, with an
  append-only event log (`backlink_events`) and raw check history (`backlink_checks`).
- **Selective monitoring:** `monitoring_enabled` gates the scheduler; links can be inventory-only
  (tracked but not polled) or `protected`. UI-added links default to monitored (opt-in checkbox).
- First-check feedback ("verification runs within ~10 minutes") after adding.

### 4.2 Inventory & import
- Manual single add, **bulk-first** add (paste `source,target` lines), and CSV **and `.xlsx`** import
  with flexible headers (generic / Ahrefs / Semrush).
- Classification: acquisition type, link attribute override, placement, format; tags (manual +
  auto-tagger for comment/PBN/forum-profile/self-built/earned).
- Warranty tracking (type/value/expiration) and value-at-risk rollups by currency.

### 4.3 Discovery
- Seed-and-crawl known referring domains (SSRF-safe, bounded crawler).
- Automatic discovery via DataForSEO (paid, any domain) or Bing (free, verified site).

### 4.4 Competitors & gap
- Track competitor domains; import their backlinks; **gap analysis** = domains linking to competitors
  but not to you, scored as prospects.

### 4.5 Link-building campaigns (BETA, env-gated `CAMPAIGNS_BETA`)
- Pipeline: gap → prospects (explainable score) → outreach draft → convert won prospect into a
  protected, monitored backlink. Per-campaign summary & rollup.

### 4.6 Post-Link Impact (BETA, gated — operator only for public launch)
- Founded on **Google Search Console**. Observes, after a link's T0 (acquisition/verification):
  crawl & indexation (URL Inspection), visibility and page-level ranking (Search Analytics),
  as before/after timelines. Correlation only, never causation. See [`../13-roadmap.md`](../13-roadmap.md).

### 4.7 Anchor text distribution
- Per-project mix: **Branded / Keyword / Naked-URL / Generic / Empty**, most-repeated anchors, and an
  over-concentration *observation*. No exact-match %, no toxic score. (`memory: anchor-distribution`.)

### 4.8 Dashboard, Overview & alerts
- `/dashboard` command center (attention engine + daily snapshot); `/backlinks` inventory.
- Email alerts (Resend) on change/loss, immediate or digest; per-destination, on by default for new
  workspaces.

### 4.9 Reporting
- CSV export of the backlink inventory + change history. **No** white-label / PDF / scheduled reports
  (explicitly out of scope — not built, not claimed).

### 4.10 Accounts, billing, admin
- Register → verify email (Resend) → 10-day no-card trial → plan.
- Plans **Starter $29 / Pro $59 / Business $99 / Agency $199** (monthly; annual ≈ 2 months free),
  with backlink/project/seat caps — single source of truth in `crates/billing .../plans.rs`. Stripe
  Checkout/Portal/webhook. See [`../20-pricing-and-trial.md`](../20-pricing-and-trial.md).
- Teams: invitations, roles (Owner > Admin > Member > Viewer).
- Platform admin backoffice (`/admin`, `is_platform_admin`, `ADMIN_EMAILS` bootstrap).
- Public marketing + programmatic-SEO surface (tool pages, docs, legal), consent-gated GA4.

## 5. Non-goals
- No causal attribution or ROI claims; no single composite SEO/toxicity score.
- No cross-tenant domain intelligence (stay within the workspace).
- No white-label/PDF/scheduled reporting (this release).
- No permanent free plan (trial only).

## 6. Success metrics
- Activation: % of new workspaces that add ≥1 monitored backlink and enable alerts.
- Time-to-detection: lag between a real change on a page and the user being alerted.
- Trial → paid conversion; monitored-backlink count per paying workspace.
