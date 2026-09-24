# 10 — Environments, deployment and configuration

**Spec refs:** §8, §14, §15, §42, §43, §44

---

## 1. `APP_ENV` gates dangerous behaviour

`APP_ENV ∈ { development, preview, production }` is read once at boot into a typed
`common::config::AppEnv`. It is **not** inferred from `NODE_ENV`, `VERCEL_ENV` or the presence of a
DSN — it is explicit, because §44 requires that preview deployments cannot do production things.

| Behaviour | development | preview | production |
|---|:--:|:--:|:--:|
| Serve API / web | ✓ | ✓ | ✓ |
| Run migrations automatically | ✗ | ✗ | ✗ |
| Process monitoring jobs | opt-in | **✗** | ✓ |
| Outbound crawling | opt-in | **✗**¹ | ✓ |
| Send email to real addresses | ✗ (mailhog) | **✗** (captured) | ✓ |
| Stripe | test keys | test keys | live keys |
| Scheduler tick accepted | ✓ | **✗** | ✓ |
| Seed/demo data endpoints | ✓ | ✓ | **✗** |

¹ A preview may crawl only an explicit allowlist of fixture hosts, for E2E tests.

These gates live in **one** place — `common::config::Guards` — and are asserted in a test that fails
if a new dangerous capability is added without a gate. A gate scattered across twelve `if` statements
is a gate that will be missed once.

```rust
// apps/worker/src/bin/worker.rs
if !cfg.guards.may_process_jobs() {
    tracing::warn!(env = ?cfg.app_env, "job processing disabled for this environment");
    return Ok(());   // start, report healthy, do nothing
}
```

The worker **starts and stays healthy** rather than crashing, so a preview environment is observable
without being dangerous.

---

## 2. Neon branches

| Environment | Neon branch | Connection |
|---|---|---|
| production | `main` | `DATABASE_URL` pooled; `DATABASE_DIRECT_URL` direct |
| preview | `preview/<pr-number>`, branched from a **sanitized** `staging` | pooled |
| development (shared) | `development` | pooled |
| local | Docker Postgres (§15 option A) or a personal Neon branch | direct |

A Vercel preview **never** receives production database credentials. Preview branches come from
`staging`, which is periodically reseeded from production with PII stripped — never from `main`
directly, because a Neon branch is copy-on-write from real data and a branch of production *is*
production data.

Each PR's Neon branch is created on open and deleted on merge/close by CI. Branches are cheap;
stale ones are not.

---

## 3. Environment variables (§43)

```bash
# ─── Core ─────────────────────────────────────────────────────────────
APP_ENV=development                 # development | preview | production
RUST_LOG=info,api=debug,crawler=debug
APP_URL=http://localhost:3000
API_URL=http://localhost:8080

# ─── Database (§6) ────────────────────────────────────────────────────
# Pooled (-pooler host) for the API. Direct for worker + migrations.
DATABASE_URL=postgres://user:pass@ep-xxx-pooler.region.aws.neon.tech/backlio?sslmode=require
DATABASE_DIRECT_URL=postgres://user:pass@ep-xxx.region.aws.neon.tech/backlio?sslmode=require
DB_MAX_CONNECTIONS=8
DB_STATEMENT_CACHE_CAPACITY=            # blank → auto: 0 when host contains -pooler, else 100

# ─── Auth (§19) ───────────────────────────────────────────────────────
SESSION_SECRET=                          # 32+ random bytes, base64
SESSION_TTL_HOURS=720
ARGON2_MEMORY_KIB=19456                  # OWASP baseline: 19 MiB, t=2, p=1
ARGON2_ITERATIONS=2
ARGON2_PARALLELISM=1

# ─── Crawler (§28) ────────────────────────────────────────────────────
CRAWLER_USER_AGENT=Backlio/1.0 (+https://backlio.app/bot)
CRAWLER_MAX_CONCURRENCY=50
CRAWLER_MAX_PER_DOMAIN=2
CRAWLER_DOMAIN_MIN_INTERVAL_MS=1000
CRAWLER_REQUEST_TIMEOUT_SECS=15
CRAWLER_TOTAL_TIMEOUT_SECS=30
CRAWLER_MAX_REDIRECTS=5
CRAWLER_MAX_RESPONSE_BYTES=2097152
CRAWLER_RESPECT_ROBOTS=true

# ─── Worker / scheduler (§10, §11) ────────────────────────────────────
WORKER_ID=                               # blank → hostname-pid-uuid
WORKER_CONCURRENCY=16
WORKER_POLL_INTERVAL_MS=1000
WORKER_JOB_LEASE_SECS=900
SCHEDULER_BATCH_SIZE=500
SCHEDULER_SECRET=                        # for POST /internal/scheduler/tick

# ─── Billing (§39) ────────────────────────────────────────────────────
STRIPE_SECRET_KEY=
STRIPE_WEBHOOK_SECRET=
STRIPE_PRICE_STARTER=
STRIPE_PRICE_PRO=
STRIPE_PRICE_AGENCY=
STRIPE_PRICE_AGENCY_PLUS=

# ─── Email (§37) ──────────────────────────────────────────────────────
EMAIL_PROVIDER_API_KEY=
EMAIL_FROM=alerts@backlio.app

# ─── Frontend (Vercel) ────────────────────────────────────────────────
NEXT_PUBLIC_APP_URL=http://localhost:3000
NEXT_PUBLIC_API_URL=http://localhost:8080
```

`.env.example` carries every key with **no** real values (§43). The config loader fails fast at boot
listing every missing or malformed variable at once — not one per restart. Secrets are typed
`secrecy::SecretString` so they cannot be logged by an accidental `{:?}`.

---

## 4. Migrations in CI (§42)

```
PR opened ──► create Neon branch preview/<pr>
          ──► sqlx migrate run   (against the PR branch, direct URL)
          ──► cargo test --workspace
          ──► Vercel preview deploy
PR merged ──► delete Neon branch
          ──► [MANUAL APPROVAL GATE]
          ──► sqlx migrate run   (production, direct URL)
          ──► deploy api + worker
```

Non-negotiable, from §42:

- Production migrations run in a **separate workflow with a required manual approval**, never as a
  side effect of a deploy.
- Application boot **verifies** the schema version and refuses to start on a mismatch. It never
  applies.
- No preview deployment holds a credential that can reach production.
- Deploy order is **migrate → API → worker**, and every migration must be backward-compatible with
  the currently running code, because there is always a window where both versions are live.

---

## 5. Local development (§15)

```bash
cp .env.example .env
docker compose up -d postgres        # local Postgres, no Neon credentials needed
cargo sqlx migrate run
cargo run -p api --bin server        # :8080
cargo run -p worker --bin worker     # separate terminal
pnpm --filter web dev                # :3000
```

Production credentials are never required locally (§15). The Neon-specific behaviours the local
Postgres does *not* reproduce — pooler semantics, prepared-statement handling, connection limits —
are covered by the opt-in integration test against a real Neon branch
([ADR-0005](adr/0005-neon-connection-strategy.md)), which skips cleanly when `NEON_TEST_POOLED_URL`
is unset. Developers are not blocked; CI still gets the coverage.
