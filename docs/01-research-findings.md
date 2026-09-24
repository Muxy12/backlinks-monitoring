# 01 — Research Findings (verified 2026-09-24)

Spec §56.2 and §56.3 require that Vercel/Rust and Neon/SQLx assumptions be **verified, not assumed**.
Everything below was checked against primary sources on 2026-09-24. Each claim is tagged:

- **[V]** verified against a primary source (linked)
- **[T]** must be verified empirically by a test we write (documentation cannot settle it)

---

## 1. Vercel support for Rust / Axum

### 1.1 Native Rust runtime — exists, public beta

Vercel ships a first-party Rust runtime for Vercel Functions, running on **Fluid compute**.
Source: <https://vercel.com/docs/functions/runtimes/rust> (page `last_updated: 2025-12-08`),
changelog <https://vercel.com/changelog/rust-runtime-now-in-public-beta-for-vercel-functions>.

Facts that shape our design:

- It is gated. The docs page carries a *"Permissions Required: The Rust runtime (Beta)"* notice. **[V]**
- The programming model is **one binary per handler file**. `Cargo.toml` must declare a `[[bin]]`
  per file in `api/`, each using `vercel_runtime::run(service_fn(handler))`. **[V]**
- Current crate versions: `vercel_runtime` **2.4.1**, Axum adapter `vercel_axum` **1.1.7**. **[V]**
- HTTP/1.1 upgrades are supported, so `axum::extract::ws::WebSocketUpgrade` works. **[V]**

The `[[bin]]`-per-handler model is the friction point. Our API is a single Axum `Router` with ~30
routes, shared auth middleware and a shared `PgPool`. Mapping that onto per-file binaries either
duplicates the whole app state per binary or collapses into one catch-all binary — at which point we
are running a whole Axum app inside a function anyway, via `vercel_axum`.

### 1.2 Container runtime — exists, runs a real Axum server

Vercel can build and run a container: add `Dockerfile.vercel` and set the service `runtime` to
`container` in `vercel.json`. Source: <https://vercel.com/kb/guide/deploy-rust-on-vercel-with-docker>

```json
"services": {
  "api": {
    "runtime": "container",
    "entrypoint": "Dockerfile.vercel"
  }
}
```

Constraints confirmed by that guide: **[V]**

- The server **must bind `0.0.0.0`** (not `127.0.0.1`) and read the `PORT` env var.
- Secrets must be read at **runtime** via `std::env`, not baked in at compile time with `env!`.
- **The filesystem is not persistent.** No local job state, no local upload spool.

### 1.3 What this means for the worker

Neither option hosts a *long-running crawler worker* well. Vercel Functions — Fluid or container —
are request-scoped autoscaling compute. They are not a place to run a process that holds a global
semaphore, per-domain rate limiters and an in-flight crawl budget for minutes at a time.

The spec is consistent with this: §4 says Vercel hosts *"frontend and web-facing application
deployment."* The worker is not web-facing. See [ADR-0002](adr/0002-api-and-worker-hosting.md).

---

## 2. Neon + Rust + SQLx

### 2.1 Pooled vs direct

Sources: <https://neon.com/docs/connect/choose-connection>,
<https://neon.com/docs/connect/connection-pooling>

- Neon fronts Postgres with **PgBouncer in `pool_mode=transaction`**, `max_client_conn=10000`,
  `default_pool_size = 0.9 * max_connections`. Pooled hostnames contain `-pooler`.
- **Use pooled by default** for application traffic.
- **Use direct** for schema migrations, `CREATE INDEX CONCURRENTLY`, `LISTEN`/`NOTIFY`, and
  anything needing stable session state.
- Direct connections are capped by `max_connections`, roughly **100–4000 depending on compute size**.

### 2.2 Features that do NOT work through the Neon pooler

This list directly constrains the job queue and scheduler design:

| Unsupported through pooler | Where we would otherwise have used it |
|---|---|
| `LISTEN` / `NOTIFY` | job-wakeup notification for the worker |
| **Session-level advisory locks** | scheduler leader election |
| Session variables (`SET` / `RESET`) | a `SET LOCAL`-style tenant guard / RLS `current_setting` |
| SQL-level `PREPARE` / `DEALLOCATE` | — (we use protocol-level) |
| `WITH HOLD` cursors | large export streaming |
| Temp tables with `PRESERVE`/`DELETE ROWS` | CSV import staging |

Consequences baked into the design:

- Scheduler leader election uses **`pg_advisory_xact_lock`** (transaction-scoped, works in
  transaction mode), never `pg_advisory_lock`.
- Worker job pickup **polls** with `FOR UPDATE SKIP LOCKED`; it does not rely on `LISTEN`/`NOTIFY`.
- CSV import staging uses a **real table** (`import_rows`), not a temp table.
- Tenant isolation is enforced in **Rust query construction**, not via a session GUC + RLS, because
  the session GUC would not survive the pooler. See [05](05-tenant-isolation.md).
- The worker connects over the **direct** endpoint (long-lived process, few connections). The API
  connects over the **pooled** endpoint.

### 2.3 Protocol-level prepared statements — the landmine

- PgBouncer supports **protocol-level** prepared statements **as of 1.22.0**; SQL-level `PREPARE`
  remains unsupported. **[V]** (<https://neon.com/docs/connect/connection-pooling>,
  <https://neon.com/blog/pgbouncer-the-one-with-prepared-statements>)
- SQLx uses protocol-level extended query with **named** statements (`sqlx_s_<n>`) and caches them
  per connection. In the field this still produces
  `prepared statement "sqlx_s_26" already exists` against transaction-mode poolers.
- The known-safe mitigation is `PgConnectOptions::statement_cache_capacity(0)`, falling back to
  unnamed prepare-per-query — safe under both session- and transaction-mode pooling, at the cost of
  re-parsing each query instead of reusing a cached plan.

**Documentation cannot settle this.** Decision: `statement_cache_capacity` is config-driven
(`DB_STATEMENT_CACHE_CAPACITY`), defaulting to `0` when the DSN host contains `-pooler` and to `100`
otherwise. An integration test (`crates/db/tests/neon_pooler.rs`) exercises a pooled connection under
concurrency and transaction churn to confirm which setting is actually required. Until that test has
run green against a real Neon branch, the conservative default stands.

### 2.4 "Double pooling"

Neon's docs warn against stacking a client-side pool on top of PgBouncer. SQLx has no poolless mode,
so the practical reading is: **keep the SQLx pool small** and do not size it as if it owned the
database. Defaults chosen: API `max_connections = 8`, worker `max_connections = 16` (direct). Both
are env-overridable and revisited under load test in Phase 7.

---

## 3. Dependency versions — current stable as of 2026-09-24

Queried from the crates.io API (`max_stable_version`).

| Crate | Version | Crate | Version |
|---|---|---|---|
| axum | 0.8.9 | tokio | 1.53.1 |
| sqlx | 0.9.0 | reqwest | 0.13.5 |
| scraper | 0.27.0 | serde | 1.0.229 |
| serde_json | 1.0.151 | tracing | 0.1.44 |
| tracing-subscriber | 0.3.23 | tower | 0.5.3 |
| tower-http | 0.7.1 | uuid | 1.26.1 |
| chrono | 0.4.45 | validator | 0.21.0 |
| thiserror | 2.0.21 | anyhow | 1.0.104 |
| argon2 | 0.6.0 | url | 2.5.8 |
| hickory-resolver | 0.26.3 | ipnet | 2.12.2 |
| axum-extra | 0.12.6 | rand | 0.10.3 |
| publicsuffix | 2.3.0 | csv | 1.4.0 |
| lettre | 0.11.23 | async-stripe | 0.41.0 |
| governor | 0.10.4 | tower_governor | 0.8.0 |
| vercel_runtime | 2.4.1 | vercel_axum | 1.1.7 |

Notes:

- **sqlx 0.9.0** shipped 2026-05-21 (0.8.6 was the prior line, 2025-05-19). We adopt 0.9 — four
  months mature, and the right starting line for a greenfield project.
- **axum 0.8.9** shipped 2026-04-14; the 0.8 line is stable and `axum-extra 0.12` matches it.
- Versions are pinned in the workspace root `[workspace.dependencies]` so every crate agrees.

---

## 4. Open items carried into implementation

| # | Item | Resolved by |
|---|---|---|
| R1 | Does SQLx 0.9 need `statement_cache_capacity(0)` against Neon's pooler? | `crates/db/tests/neon_pooler.rs` (Phase 1) |
| R2 | Is the native Rust runtime beta enabled on the target Vercel account? | Ask account owner; ADR-0002 does not depend on it |
| R3 | Cold-start cost of the container runtime for the API under Fluid | Phase 7 load test |
| R4 | Neon compute size vs `max_connections` headroom for the worker | Phase 7 |

---

## Sources

- [Using the Rust Runtime with Vercel functions](https://vercel.com/docs/functions/runtimes/rust)
- [Rust runtime now in public beta for Vercel Functions](https://vercel.com/changelog/rust-runtime-now-in-public-beta-for-vercel-functions)
- [Deploy Rust on Vercel with Docker](https://vercel.com/kb/guide/deploy-rust-on-vercel-with-docker)
- [Neon — Choosing your connection method](https://neon.com/docs/connect/choose-connection)
- [Neon — Connection pooling](https://neon.com/docs/connect/connection-pooling)
- [Neon — PgBouncer: The one with prepared statements](https://neon.com/blog/pgbouncer-the-one-with-prepared-statements)
- [vercel_axum on docs.rs](https://docs.rs/vercel_axum/latest/vercel_axum/)
- [vercel-community/rust](https://github.com/vercel-community/rust)
