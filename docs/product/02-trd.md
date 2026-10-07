# Technical Requirement Document — LinkPulz

Companion to the [PRD](01-prd.md). Describes the system that delivers it. Deeper per-topic specs live
in the numbered `docs/` set; this document is the map and the requirements, not a re-copy.

---

## 1. Architecture

**Modular monolith in a Cargo workspace** (ADR [`0001`](../adr/0001-modular-monolith-cargo-workspace.md)),
Rust **edition 2024**. Three binaries over shared crates:

```
apps/
  api      Axum HTTP API (one router, shared state)
  worker   crawler + scheduler + job dispatch (long-lived; NOT serverless)
  web      Next.js 15 App Router (React 19) marketing + app
crates/
  common   enums & shared types (Role, Plan live here/billing)
  core     backlio-core — PURE domain logic: state machine, scoring, impact,
           anchors, monitoring policy, url_normalize. No I/O, no clock, no sqlx.
  db       sqlx repositories, entities, migrations, OrgScope (tenant key)
  crawler  SSRF-safe fetcher, link matching, politeness
  jobs     JobKind + enqueue/claim
  auth     sessions, tokens, crypto helpers
  billing  Stripe + plan catalog (plans.rs = single source of truth)
  notifications  Resend email channel
```

See [`../02-monorepo.md`](../02-monorepo.md).

## 2. Platform & hosting

| Concern | Choice |
|---|---|
| Database | **Neon PostgreSQL** (pooled + direct split, ADR [`0005`](../adr/0005-neon-connection-strategy.md)) |
| API + worker | **Render** (`backlio-api` web service; worker process). Worker cannot run on Vercel (ADR [`0002`](../adr/0002-api-and-worker-hosting.md)). |
| Web | **Vercel** (proxies `/v1/*` to the API; cookie-domain proxy) |
| Scheduler tick | **GitHub Actions cron** (`monitor-tick.yml`, every 10 min) → `POST /internal/tick` with `x-scheduler-secret` |
| Migrations | `migrate-production.yml` (manual `workflow_dispatch`, applied to Neon) |

Current live URLs and deploy gotchas: memory `deployment-free-tier`; [`../17-deployment.md`](../17-deployment.md).

## 3. Load-bearing technical decisions

1. **Tenant isolation in Rust, not Postgres RLS** (ADR [`0004`](../adr/0004-tenant-isolation-in-rust.md)).
   `OrgScope` has no public constructor — only `authorize_org` mints it after a membership lookup —
   and it is the only key that opens tenant-scoped repositories. Every query filters
   `organization_id`. See [`../05-tenant-isolation.md`](../05-tenant-isolation.md).
2. **PostgreSQL job queue, no Redis** (ADR [`0003`](../adr/0003-postgres-job-queue.md)), `FOR UPDATE
   SKIP LOCKED` claim; polling, because Neon's pooler has no `LISTEN`/`NOTIFY`. See [`../07-job-queue.md`](../07-job-queue.md).
3. **Pure core.** All decisions are pure functions (state machine, scoring, impact, anchors) so they
   are exhaustively unit-tested with no DB/network. `Absent` is the only observation that yields
   `lost`.
4. **SQL must be `&'static str` literals.** An injection-audit lint rejects `format!()`-built SQL;
   dynamic queries use `sqlx::QueryBuilder` with bound params only.

## 4. External integrations (all env-gated, inert until configured)

| Integration | Env | Use |
|---|---|---|
| Stripe | `STRIPE_*` | Checkout / Billing Portal / webhook (`memory: stripe-billing`) |
| Resend | `EMAIL_PROVIDER_API_KEY`, `EMAIL_FROM`, `RESEND_AUDIENCE_ID`, `APP_URL` | alert + auth emails, waitlist→Audience sync |
| Google Search Console | `GOOGLE_OAUTH_CLIENT_*`, `GSC_TOKEN_KEY` | Post-Link Impact (OAuth, tokens AES-256-GCM at rest) |
| Turnstile | `TURNSTILE_*` | anti-spam on register / forgot / backlink-check |
| Discovery | DataForSEO / Bing creds | automatic backlink discovery |
| GA4 | `NEXT_PUBLIC_GA_ID` | consent-gated analytics (`G-NMT0YJEBRS`) |

## 5. Non-functional requirements

- **Security:** strict CSP (nonce + strict-dynamic; no unsafe-inline scripts), tenant isolation as a
  type, secrets in env only, tokens encrypted at rest. Risk register: [`../11-security-risks.md`](../11-security-risks.md).
- **Crawler economics & cost:** opening any screen is a **DB-only read** — zero external calls on
  open. External calls happen only in scheduled jobs, deduped per target/property. [`../12-observability.md`](../12-observability.md).
- **Multi-tenancy:** every row is org-scoped; IDOR-safe (NotFound, never Forbidden, for non-members).
- **Cost posture:** runs on free tiers today; a real launch moves Render/Neon to paid.

## 6. Build, test & CI gates

Local box **cannot** compile `db`/`api`/`worker` (dlltool/getrandom). Locally runnable:
`cargo test -p backlio-core`, `cargo fmt --all --check`, `cargo metadata --locked`,
`node tools/schema-check/verify.mjs`, web `typecheck`/`build`. **CI is the Rust gate.**

CI jobs: `rust` (fmt → clippy `-D warnings` → test → build), `web`, `migrations`, `schema-check`,
`security-audit` (cargo-audit + npm advisory scan). All must be green before merge.

## 7. API surface

~110 endpoints under `/v1/*`, plus `/health`, `/ready`, `/internal/tick`, `/v1/billing/webhook`,
`/v1/gsc/oauth/callback`. Conventions and the full list: [`../09-api.md`](../09-api.md) and the route
map in [`03-app-flow.md`](03-app-flow.md). All tenant endpoints are nested under
`/v1/organizations/{org_id}/…` and require an `OrgScope`.
