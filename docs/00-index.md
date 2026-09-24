# Backlio — architecture documentation

Backlink monitoring and link-building operations platform.
**Rust + Axum · Neon PostgreSQL · Next.js on Vercel.**

This is the deliverable for spec §56 ("first task for the coding agent"): analyze, verify, design,
document — *then* build.

---

## Documents

| # | Document | Spec §56 item |
|---|---|---|
| [01](01-research-findings.md) | **Research findings** — verified Vercel/Rust and Neon/SQLx facts, current crate versions | 1, 2, 3 |
| [02](02-monorepo.md) | Monorepo layout | 4 |
| [03](03-database-schema.md) | Neon PostgreSQL schema | 5 |
| [04](04-erd.md) | Entity relationship diagram | 6 |
| [05](05-tenant-isolation.md) | Tenant isolation | 7 |
| [06](06-crawler.md) | Crawler design, SSRF, dedup, politeness | 8 |
| [07](07-job-queue.md) | PostgreSQL job queue and scheduler | 9 |
| [08](08-backlink-state-machine.md) | Backlink state machine | 10 |
| [09](09-api.md) | API routes | 11 |
| [10](10-environments.md) | Deployment and environment separation | 12, 13 |
| [11](11-security-risks.md) | Security risk register | 14 |
| [12](12-observability.md) | Observability and crawler economics | — |
| [13](13-roadmap.md) | Implementation roadmap | 15 |
| [14](14-design-system.md) | Design system & frontend, from the supplied homepage | — |
| [16](16-local-toolchain.md) | Local Rust toolchain setup on this Windows box | — |

## Decisions

| ADR | Decision |
|---|---|
| [0001](adr/0001-modular-monolith-cargo-workspace.md) | Modular monolith in a Cargo workspace |
| [0002](adr/0002-api-and-worker-hosting.md) | API on Vercel's container runtime; **worker off Vercel** |
| [0003](adr/0003-postgres-job-queue.md) | PostgreSQL job queue, no Redis |
| [0004](adr/0004-tenant-isolation-in-rust.md) | Tenant isolation in Rust, **not** Postgres RLS |
| [0005](adr/0005-neon-connection-strategy.md) | Neon pooled/direct split and pool sizing |

---

## The five findings that changed the design

Each of these came out of verification, not assumption — which is the point of §56.

1. **Neon's pooler does not support session-level advisory locks, `LISTEN`/`NOTIFY`, session
   variables or temp tables.** This rules out RLS-via-session-GUC (→ ADR-0004), rules out
   notify-based job wakeup (→ polling, ADR-0003), forces `pg_advisory_xact_lock` for scheduler
   leader election, and forces a real staging table for CSV imports.

2. **SQLx's prepared-statement cache is a live risk against a transaction-mode pooler.** Documented
   support (PgBouncer ≥ 1.22) does not match reported field behaviour. We default to the setting
   that cannot break and ship a test to tell us whether we can raise it — open item **R1**. No claim
   is made until that test runs green ([01 §2.3](01-research-findings.md#23-protocol-level-prepared-statements--the-landmine)).

3. **Vercel's native Rust runtime is real but is `[[bin]]`-per-handler and in gated beta.** Our API
   is one Axum router with shared state, so the container runtime is the better fit — with the
   native-runtime adapter kept as a 30-line alternate binary, not a rewrite
   ([ADR-0002](adr/0002-api-and-worker-hosting.md)).

4. **The crawler worker cannot live on Vercel.** It holds semaphores and rate-limit state for
   minutes. It runs as a container elsewhere. This is consistent with the spec, which scopes Vercel
   to web-facing deployment (§4).

5. **`Absent` must be the only observation that can produce `lost`.** Encoded as a type in
   [08](08-backlink-state-machine.md) and enforced by a property test, because §24/§57.10 is the
   product's core promise and a `Vec<if>` will eventually get it wrong.

---

## Status

**Phase 0 (design) complete. Phase 1 (foundation) is next** — see [13](13-roadmap.md).

Open items carried forward: **R1** (SQLx statement cache vs Neon pooler — resolved by a Phase 1
test), **R2** (Vercel Rust-runtime beta access — blocks nothing), **R3**/**R4** (cold start and
connection headroom — Phase 7 load test).
