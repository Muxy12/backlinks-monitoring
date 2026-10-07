# Backend / Data Schema — LinkPulz

The data model behind the product. The authoritative DDL is the ordered migration set
(`migrations/0001 … 0040`); the narrative schema and ERD are [`../03-database-schema.md`](../03-database-schema.md)
and [`../04-erd.md`](../04-erd.md). This document is the grouped map and the invariants.

**Engine:** Neon PostgreSQL. **Convention:** migrations are **additive and ordered**; enums/extensions
first. Access is through sqlx repositories in `crates/db`, never raw from the API.

---

## 1. Tables by domain (40)

### Identity & sessions
`users` · `sessions` · `auth_tokens` — registration, verified email, password hash, platform-admin
flag; session tokens; email-verify / password-reset tokens.

### Tenancy & teams
`organizations` · `organization_members` (role) · `organization_invitations` — the tenant boundary.
Every tenant-scoped table carries `organization_id`. Suspension lives on `organizations`.

### Workspace resources
`clients` (agency grouping) · `projects` (one domain, `check_frequency_secs`, timezone, status).

### Backlinks & lifecycle (the core)
`backlinks` (create/observe projection: `*_url` + `*_url_normalized`, `expected_` vs `detected_`
anchor/rel, status, cost/currency, warranty, subject own/competitor, tier, campaign_id, acquisition/
attribute/placement classification, monitoring flags) · `backlink_checks` (raw check history) ·
`backlink_events` (append-only lifecycle events) · `link_occurrences` (per-occurrence, suspected-lost).

### Tags
`tags` · `backlink_tags` — manual + auto-tagger (comment/PBN/forum-profile/self-built/earned).

### Jobs & scheduling
`jobs` — the Postgres queue (`FOR UPDATE SKIP LOCKED`, priority, lease/reap). See [`../07-job-queue.md`](../07-job-queue.md).

### Import
`imports` · `import_rows` — staged CSV/`.xlsx` import (real staging table; Neon pooler has no temp tables).

### Discovery, competitors & tiers
`discovery_seeds` · `project_competitors` — seed-and-crawl + competitor tracking and gap analysis;
backlink `tier` / `parent_backlink_id` carry Tier-1/Tier-2 structure.

### Link-building campaigns (BETA)
`campaigns` · `campaign_targets` · `prospects` · `publishers` — pipeline from gap → prospect →
outreach → converted protected backlink.

### Billing & plans
`subscriptions` · `usage_records` · `stripe_events` · `trial_email_log` — Stripe subscription state,
usage vs plan caps, idempotent webhook ledger, trial-lifecycle email log. Plan catalog itself is
**code** (`plans.rs`), not a table. See [`../20-pricing-and-trial.md`](../20-pricing-and-trial.md).

### Notifications & alerts
`alert_destinations` · `alert_deliveries` · `notifications` · `notification_preferences` — email
channels (immediate/digest), delivery log, in-app notifications.

### Search Console & Post-Link Impact (BETA)
`gsc_connections` (OAuth, refresh/access tokens **AES-256-GCM** at rest, mapped `site_url`) ·
`gsc_daily` (per-URL Search Analytics time series) · `gsc_url_inspection` (crawl/indexation) ·
`backlink_impact` (one row per eligible backlink: T0 + source, status). Metrics are computed at read
time from `gsc_daily` — no external call on open.

### Analytics, growth & trust
`analytics_events` · `waitlist_signups` · `feedback` · `audit_logs` · `daily_backlink_snapshot`
(overview growth series).

## 2. Cross-cutting invariants

1. **Tenant column everywhere.** Every tenant row has `organization_id`; repositories filter it via
   `OrgScope` ([`../05-tenant-isolation.md`](../05-tenant-isolation.md)).
2. **Normalized URL columns.** `source_url_normalized` / `target_url_normalized` are written with the
   same `url_normalize` logic the crawler uses, so a discovered link and a stored link compare on
   identical terms (prevents false "target changed").
3. **Append-only history.** `backlink_events` and `backlink_checks` are never mutated — the timeline
   and Post-Link Impact are recomputable from raw rows.
4. **Money by currency.** Costs/value are always grouped by currency, never summed across currencies.
5. **Enums as text at the edge.** Status/rel/role are PG enums cast to `text` in projections so no
   custom sqlx type mapping is needed.

## 3. Migration ledger (0001–0040)

`0001` extensions+enums · `0002` identity · `0003` tenancy · `0004` operations · `0005` backlinks ·
`0006` history · `0007` tags · `0008` jobs · `0009` imports · `0010` billing · `0011` notifications ·
`0012` audit · `0013–0016` detected flags / fk fix / link_type / target_final_url · `0017`
link_occurrences · `0018` suspected-lost + events · `0019` competitors · `0020` tiers · `0021`
discovery_seeds · `0022` platform_admin · `0023` org suspend · `0024–0026` alert destinations/kind/
digest · `0027` plan respec · `0028` analytics_events · `0029` trial_email_log · `0030–0032` waitlist
(+confirmation/unsubscribe) · `0033` feedback · `0034` backlink classification · `0035` monitoring ·
`0036` link-building · `0037` daily snapshot · `0038` GSC · `0039` backlink_impact · `0040` GSC URL
inspection.

Applied to Neon via `migrate-production.yml` (manual dispatch). New work adds `0041+`, additive only.
