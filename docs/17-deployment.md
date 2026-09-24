# 17 — Deployment (API + worker)

The Rust services are **long-running** and do not belong on Vercel (ADR-0002).
This runbook deploys them to **Fly.io** as one app with two process groups, with
the schema on **Neon** and the marketing/app frontend on **Vercel**.

```
              ┌───────────────┐        ┌──────────────────────────┐
  browser ──▶ │ Vercel (web)  │ ─────▶ │ Fly: backlio             │
              │ Next.js       │  API   │  web    process → server │ ─┐
              └───────────────┘        │  worker process → worker │  │ SQL
                                       └──────────────────────────┘  ▼
                                                        ┌────────────────────┐
                                                        │ Neon Postgres      │
                                                        │  pooled  (API)     │
                                                        │  direct  (worker,  │
                                                        │           migrate) │
                                                        └────────────────────┘
```

## Artifacts in the repo

- **`Dockerfile`** — one image with `server`, `worker`, `scheduler` binaries.
- **`fly.toml`** — the `web` (public) and `worker` (internal) process groups.
- **`.github/workflows/migrate-production.yml`** — the gated schema migration.

## Endpoint rules (do not mix these up)

| Consumer | URL | Why |
|---|---|---|
| API (`server`) runtime | **pooled** `…-pooler…` | many short connections; sqlx statement cache auto-disabled for `-pooler` |
| worker / scheduler | **direct** (non-pooler) | long-lived connections, advisory locks, no PgBouncer limits |
| migrations | **direct** | DDL + advisory locks (ADR-0005) |

## One-time setup

Install and sign in to the Fly CLI (`fly auth login`), then from the repo root:

```bash
fly apps create backlio            # or: fly launch --no-deploy (reuses fly.toml)
```

Set secrets — **names only shown here; never commit the values**. Map the Neon
URLs correctly (pooled vs direct):

```bash
fly secrets set \
  DATABASE_URL="<neon POOLED url, ...-pooler...>" \
  DATABASE_DIRECT_URL="<neon DIRECT url, no -pooler>" \
  SESSION_SECRET="<random 32+ bytes>" \
  SESSION_TTL_HOURS="720" \
  STRIPE_SECRET_KEY="<sk_live_...>" \
  STRIPE_WEBHOOK_SECRET="<whsec_...>" \
  STRIPE_PRICE_STARTER="<price_...>" \
  STRIPE_PRICE_PRO="<price_...>" \
  STRIPE_PRICE_AGENCY="<price_...>" \
  STRIPE_PRICE_AGENCY_PLUS="<price_...>" \
  EMAIL_PROVIDER_API_KEY="<...>" \
  EMAIL_FROM="alerts@yourdomain.com" \
  SCHEDULER_SECRET="<random>"
```

Non-secret config (`APP_ENV=production`, `PORT`, `RUST_LOG`, `DB_MAX_CONNECTIONS`)
is already in `fly.toml`'s `[env]`. `APP_ENV=production` is what lets the worker
actually process jobs and crawl (the env guard, docs/10 §1); a non-production
value makes it idle.

## Apply the schema (once, and after any migration)

Migrations are a deliberate, gated step — never a side effect of a deploy
(ADR-0005). Add the **direct** URL as the `DATABASE_DIRECT_URL` secret of the
`production` GitHub Environment, then run the **Migrate production** workflow
(Actions → *Migrate production* → type `migrate production`). It runs
`sqlx migrate run` against the direct endpoint.

> The initial 13 migrations are already applied to the current Neon database
> (recorded in `_sqlx_migrations` with the exact sqlx SHA-384 checksums), so the
> services will pass their boot-time `verify()` immediately. Re-run the workflow
> only when new migrations land.

## Deploy

```bash
fly deploy
```

This builds the image, starts one `web` machine (public, health-checked on
`/health`) and one `worker` machine (internal). Watch it:

```bash
fly logs
fly status
fly ssh console -C "/app/server --help" # sanity (binary present)
```

The API boot logs `schema verified` when the migration state matches the binary;
a mismatch logs a warning (it does not hard-fail readiness, but fix it before
trusting writes).

## Scheduler (optional)

The `worker` already runs the scheduler on a 30s timer, so due checks are
enqueued without anything else. If you want an independent cron as well, run the
one-shot `scheduler` binary on a schedule, e.g. a Fly scheduled Machine:

```bash
fly machine run . --schedule=hourly --command "/app/scheduler" \
  --env APP_ENV=production
```

(The advisory lock makes concurrent scheduler runs safe — a double fire is a
no-op.)

## Frontend (Vercel)

Deploy `apps/web` to Vercel and set:

- `NEXT_PUBLIC_API_URL` = the Fly API origin (e.g. `https://backlio.fly.dev`).
- `NEXT_PUBLIC_APP_URL` = the public web origin.

CORS/cookies: the API sets `Secure`, `SameSite=Lax` cookies in production, so the
web app and API should share a parent domain (e.g. `app.example.com` +
`api.example.com`) for the session cookie to travel. If they are on unrelated
domains, front both behind one domain or add an explicit CORS+cookie strategy
before launch.

## Point Stripe at the webhook

Create a Stripe webhook endpoint at `https://<api-origin>/v1/billing/webhook`
and set its signing secret as `STRIPE_WEBHOOK_SECRET`. Without the secret the
route returns 404 by design.

## Scaling notes

- Start with **one** `worker` machine: per-domain rate limiting and the
  advisory-locked scheduler are correct at one instance; scale out only after
  verifying the lease/reap behaviour under load.
- Scale the API horizontally freely (`fly scale count web=N`) — it is stateless.
- Keep `min_machines_running = 1` for `web` so the API never cold-starts on a
  health check.

---

## Free-tier deployment (Render + Vercel + cron)

Same structure (front / back / DB separate), zero cost, no always-on worker.

- **Frontend** → **Vercel** (Hobby, free). Import the repo, set the root to
  `apps/web`. Because the front and API are on different domains, use the
  **same-origin proxy** so the session cookie survives: set
  `API_PROXY_TARGET` = the API origin (e.g. `https://backlio-api.onrender.com`)
  and `NEXT_PUBLIC_API_URL` = `/api`. Next then forwards `/api/*` to the API
  server-side and the cookie stays first-party (no CORS, no custom domain
  needed). Point Stripe's webhook at `https://<vercel-app>/api/v1/billing/webhook`.
- **API** → **Render** free web service via `render.yaml` (Docker). Set the
  `sync:false` secrets in the Render dashboard (Neon pooled `DATABASE_URL`,
  direct `DATABASE_DIRECT_URL`, `SESSION_SECRET`, `SCHEDULER_SECRET`, Stripe/email).
- **Monitoring** → no worker process. `POST /internal/tick` (auth:
  `x-scheduler-secret: $SCHEDULER_SECRET`) does one round — schedule due checks,
  process a batch (crawl), warranty sweep. `.github/workflows/monitor-tick.yml`
  calls it every ~10 min (also keeps the free API awake). Enable it by setting
  the repo secrets `API_URL` and `SCHEDULER_SECRET`.
- **DB** → Neon (already migrated to 0014).

Trade-offs: the free API sleeps when idle (first hit after a long gap is slow;
the tick limits this), and monitoring runs in ~10-minute waves rather than a
tight loop — fine for checks that default to weekly. Move the worker back to an
always-on host (Fly/Render paid/VM) for higher cadence.
