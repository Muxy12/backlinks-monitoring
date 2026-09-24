# ADR-0001 — Modular monolith in a Cargo workspace

- **Status:** Accepted
- **Date:** 2026-09-24
- **Spec refs:** §16, §17, §57.15

## Context

We need an API and a crawler worker that share domain types, database access, authorization rules
and job semantics. The temptation at this stage is either (a) one giant binary, or (b) microservices
split by noun.

## Decision

One Cargo workspace. Domain logic lives in **library crates**; deployables are **thin binaries**.

```
crates/core          domain types, backlink state machine, value objects — no I/O
crates/db            SQLx pool, migrations runner, repositories (tenant-scoped)
crates/crawler       URL normalization, SSRF guard, fetch, parse, CrawlResult
crates/jobs          job queue, claim/complete/retry, job handlers
crates/auth          Argon2id, sessions, CSRF, membership/role checks
crates/billing       Stripe plans, entitlements, webhook handling
crates/notifications alert fan-out (in-app, email), channel trait for Slack/webhook later
crates/common        config, error types, tracing setup, IDs, clock

apps/api             Axum router + HTTP binaries (server, vercel adapter)
apps/worker          worker loop + scheduler binaries
apps/web             Next.js frontend
```

Rules:

1. `crates/core` has **no** `sqlx`, no `reqwest`, no `axum`. It is pure domain logic and is trivially
   unit-testable. The backlink state machine lives here.
2. `apps/*` contain wiring, not rules. A route handler validates input, calls a crate, maps the
   result to HTTP. If a handler contains a business rule, that rule is in the wrong place.
3. Crates depend downward only. `crates/common` is the leaf everything may use. No cycles.
4. The API and the worker are separate binaries but the same codebase. They are split because they
   have **different runtime shapes** (request-scoped vs long-running), not because they are
   different "services."

## Consequences

- We can move the worker to a different host without touching domain code (this matters — see
  ADR-0002).
- `cargo test -p core` runs the whole state machine suite with no database.
- The cost is workspace ceremony up front: a root `Cargo.toml` with
  `[workspace.dependencies]` pinning every shared version so crates cannot drift.
- If we later need to split a service out for real, the crate boundary is already the seam.
