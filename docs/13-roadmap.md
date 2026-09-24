# 13 — Implementation roadmap

**Spec refs:** §3, §55, §56

Every phase has **exit criteria**, and a phase is not done until they have actually been observed
passing (§56: *"Do not claim something works unless the relevant checks have actually passed"*).

The gate after every milestone is the same:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
pnpm --filter web lint && pnpm --filter web typecheck && pnpm --filter web test
sqlx migrate run && sqlx migrate info
```

---

## Phase 0 — Design · **complete**

Deliverables: [01](01-research-findings.md) research verification · [02](02-monorepo.md) ·
[03](03-database-schema.md) · [04](04-erd.md) · [05](05-tenant-isolation.md) ·
[06](06-crawler.md) · [07](07-job-queue.md) · [08](08-backlink-state-machine.md) ·
[09](09-api.md) · [10](10-environments.md) · [11](11-security-risks.md) ·
[12](12-observability.md) · ADRs 0001–0005.

---

## Phase 1 — Foundation

Monorepo · Cargo workspace · `common` config + tracing · `db` pool + migrations · `core` types ·
Axum skeleton with middleware (request id, body limit, CSRF, rate limit, error mapping) · auth
(register, login, logout, verify, reset, sessions) · organizations + members + invitations ·
clients · projects · `OrgScope` · Next.js app with auth flows and app shell · Docker Postgres ·
CI · first Vercel deploy.

**Started early (before the Rust toolchain — blocked on local disk space):**

- [x] All 12 SQLx migrations written (`/migrations`), covering every spec §41 table.
- [x] Schema verified in PGlite (Postgres 17 WASM) via `tools/schema-check` — 54 assertions green,
      covering composite-FK tenant isolation, business-rule CHECKs, idempotency indexes, the
      scheduler query plan and the job claim query. Caught and fixed a NULL-logic bug in the
      warranty constraint. **Not** a substitute for the SQLx/Neon tests below.
- [x] Rust toolchain installed (1.98.1, GNU host — [docs/16](16-local-toolchain.md)) and the
      **`crates/core` state machine implemented and compiler-verified**: `cargo fmt --check`,
      `cargo clippy -D warnings` and `cargo test` all green, 18 tests including the property test
      `only_absent_can_ever_produce_lost` that mechanically enforces §24 (docs/08 §5). This is the
      Phase 2 state-machine deliverable, pulled forward because it is pure logic and needs no
      database.

**Exit criteria**

- [ ] Full gate green
- [ ] A user can register, verify, log in, create an org, invite a member, create a client and a project — **in a deployed preview**, not only locally
- [ ] `apps/api/tests/tenant_isolation.rs` passes for every route that exists so far
- [ ] `crates/db/tests/neon_pooler.rs` has **run against a real Neon branch**, resolving open item R1
- [ ] Migrations apply cleanly from empty in CI
- [ ] Argon2 parameters benchmarked, not guessed

## Phase 2 — Backlink core

Backlink CRUD + filtering/pagination · `crates/crawler` with the full SSRF guard · URL normalizer ·
HTML parse + link matching · `crates/core` state machine · manual "check now" (enqueued, §36) ·
`backlink_checks` + `backlink_events` · backlink list and detail UI with the §30 timeline.

**Exit criteria**

- [ ] Every fixture in [06 §7](06-crawler.md#7-test-fixtures) passes
- [~] The SSRF suite passes, **including redirect-to-private and DNS-rebinding** — the pure
      **address classifier** is done and tested in `crates/core::net` (23 cases incl. v4-mapped and
      6to4 unwrap, metadata, all private/reserved ranges); DNS resolution, address pinning and the
      redirect-hop re-check remain for `crates/crawler` (needs a C compiler for rustls — see
      [16](16-local-toolchain.md))
- [x] The state machine property test holds: no sequence of non-`Absent` observations reaches `lost`
      (`crates/core`, done early — 18 tests green)
- [x] The URL normalizer folds harmless http/https/www variation without false `TARGET_CHANGED`
      (`crates/core::url_normalize`, done early)
- [ ] "Check now" enqueues and returns 202; no crawling happens inside a request
- [ ] The timeline renders a real multi-event history

## Phase 3 — Monitoring

`jobs` table · claim/complete/retry/reap · worker binary · scheduler with
`pg_advisory_xact_lock` · source-URL dedup grouping · global + per-domain rate limiting ·
`robots.txt` cache · automatic monitoring end to end.

**Exit criteria**

- [ ] 10 000 seeded backlinks monitored end to end without manual intervention
- [ ] `crawl_dedup_ratio` measurably > 1 on seeded overlapping sources
- [ ] Per-domain concurrency verified by test, not by inspection
- [ ] A killed worker's jobs are reaped and completed by another worker
- [ ] `EXPLAIN ANALYZE` on the scheduler query confirms an index range scan (§12)

## Phase 4 — Operations

Campaigns with rollups · publishers with lifespan/removal-rate metrics · cost tracking ·
warranty tracking + expiry job · Value at Risk · the §34 dashboard.

**Exit criteria**

- [ ] Value at Risk grouped **by currency** and labelled as recorded acquisition cost, never as loss (§33)
- [ ] Warranty states correct across `none`/`days`/`months`/`lifetime`, including the §32 worked example
- [ ] Dashboard queries confirmed against the partial indexes

## Phase 5 — Imports

Generic CSV · Ahrefs format · Semrush format · mapping UI · preview · validation · duplicate
detection · async chunked import · initial check scheduling · onboarding flow (§49).

**Exit criteria**

- [ ] A 50 000-row CSV imports without blocking a request or starving other tenants' jobs
- [ ] Duplicates detected against `backlinks_dedupe_uq`
- [ ] Re-running an interrupted import creates no duplicate rows (idempotency, §10)
- [ ] Onboarding reaches a first result screen (§49) — time-to-value measured, not assumed

## Phase 6 — Billing

Stripe Checkout + Customer Portal · the four plans (§39) · idempotent webhooks · usage metering ·
plan limit enforcement · upgrade/downgrade/cancel.

**Exit criteria**

- [ ] Webhook replay produces no second effect
- [ ] Out-of-order webhooks converge to the correct entitlement
- [ ] Exceeding a plan limit blocks **new** backlinks without stopping monitoring of existing ones
- [ ] Downgrade below current usage handled explicitly, with a defined customer-facing behaviour

## Phase 7 — Production hardening

SSRF audit · tenant isolation audit · Neon query optimization with `EXPLAIN ANALYZE` · crawler load
test · full observability + alerts · backup/restore rehearsal · `cargo deny` · the §11 checklist.

**Exit criteria**

- [ ] Every item in the [Phase 7 audit checklist](11-security-risks.md#phase-7-audit-checklist-55) ticked
- [ ] Cost per 1000 monitored backlinks measured (§53) and compared against plan pricing
- [ ] Restore from backup rehearsed end to end, not merely configured
- [ ] Load test at 10× expected launch volume

---

## Explicitly out of scope for MVP

Per §3, §51 and §52, and stated here so it is a decision rather than an omission: global backlink
discovery · a proprietary web index · competitor/backlink gap analysis · DR/DA-style third-party
metrics · AI recommendations · Google indexation status (§51 — we will not fabricate it, and we will
not promise to get links indexed) · white-label · Slack/Discord/webhook channels (the trait is
defined in Phase 4; implementations come later).

The product must be useful without any of these (§57, Final Objective).
