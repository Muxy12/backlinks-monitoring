# ADR-0005 — Neon connection strategy

- **Status:** Accepted
- **Date:** 2026-09-24
- **Spec refs:** §6, §13, §42
- **Research:** [01 §2](../01-research-findings.md#2-neon--rust--sqlx)

## Decision

Two DSNs, three consumers, conservative pools.

| Consumer | DSN | Pool max | Why |
|---|---|---|---|
| API (`apps/api`) | `DATABASE_URL` (pooled, `-pooler` host) | 8 | Many short transactions, many instances. Neon's PgBouncer absorbs the fan-out. |
| Worker (`apps/worker`) | `DATABASE_DIRECT_URL` | 16 | Few long-lived processes; wants stable connections for `SKIP LOCKED` batches. |
| Migrations (CI / `sqlx migrate`) | `DATABASE_DIRECT_URL` | 1 | DDL, `CREATE INDEX CONCURRENTLY`, advisory locks. Must not go through the pooler. |

### Pool sizing

Neon warns against "double pooling." SQLx has no poolless mode, so the rule we adopt is: **the SQLx
pool is a local work-queue, not a connection reservoir.** API instances get 8. That is a starting
point tied to open item R3/R4, not a tuned number, and it is env-overridable
(`DB_MAX_CONNECTIONS`).

Also set:

- `acquire_timeout = 10s` — fail fast and shed load rather than pile up.
- `idle_timeout = 10 min`, `max_lifetime = 30 min` — recycle so a Neon compute restart or scale
  event does not leave us holding dead sockets.
- `min_connections = 0` on the API — a scaled-to-zero instance should hold nothing. The worker uses
  `min_connections = 2` to avoid reconnect churn on every poll.

### Prepared statement cache — the unresolved one

`DB_STATEMENT_CACHE_CAPACITY` defaults to **0 when the DSN host contains `-pooler`**, else 100.

Rationale in [01 §2.3](../01-research-findings.md#23-protocol-level-prepared-statements--the-landmine):
PgBouncer ≥1.22 supports protocol-level prepared statements, which *should* make SQLx's named
statements safe, but the `prepared statement "sqlx_s_N" already exists` failure is well attested
against transaction-mode poolers. We default to the setting that cannot break, and we ship a test
that tells us whether we can raise it:

```
crates/db/tests/neon_pooler.rs   # requires NEON_TEST_POOLED_URL; skipped otherwise
```

The test drives concurrent transactions over the pooled endpoint with a non-zero cache and asserts
no `sqlx_s_` collision. **Until that test has run green against a real Neon branch, no claim is made
about which setting is correct** — only that the default is the safe one.

### Migrations

- Run with `sqlx migrate run` against `DATABASE_DIRECT_URL`, as an **explicit, gated CI step** —
  never automatically from an application boot, and never from a preview deployment
  ([10](../10-environments.md)).
- `sqlx::migrate!()` is embedded in `crates/db` so the binary can verify (not apply) that the schema
  version it expects is present, and refuse to start on a mismatch.
- Every migration is forward-only and additive where possible. Destructive changes are split across
  two deploys (add + backfill, then drop) so a rollback never loses data.

## Consequences

- The connection strategy is testable rather than assumed, which is exactly what §13 asks for.
- Defaulting the statement cache off costs re-parse time per query on the API path. That is a real
  but small cost, and it is the correct trade until measured otherwise.
- The worker's direct connections consume `max_connections` headroom on the Neon compute. At MVP
  worker counts (1–3 processes × 16) this is comfortable; it is tracked as open item R4.
