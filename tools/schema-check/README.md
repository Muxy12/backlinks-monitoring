# Schema smoke test (toolchain-free)

Applies every file in `/migrations` into **PGlite** — real PostgreSQL 17 compiled to WASM — and
asserts the design's load-bearing invariants. It needs only Node; no Rust toolchain, no Docker, no
Neon. That makes it a fast first line of defence, and it is how the warranty-constraint NULL bug in
`0005_backlinks.sql` was caught before any Rust existed.

```bash
cd tools/schema-check
npm install
npm run check          # or: node verify.mjs [path/to/migrations]
```

Exit code is non-zero if any assertion fails.

## What it checks

- All 24 spec §41 tables and all 10 enum types are created, migrations apply in order.
- **Tenant isolation layer 3** (composite FKs): a backlink/tag/project cannot reference another
  org's project/campaign/tag/client — [docs/05](../../docs/05-tenant-isolation.md),
  [ADR-0004](../../docs/adr/0004-tenant-isolation-in-rust.md).
- **Business-rule CHECK constraints**: cost-without-currency, currency format, warranty value
  (including the NULL three-valued-logic case), archived-not-scheduled, HTTP status range, check
  frequency floor, invitations cannot grant ownership, session hashes are 32 bytes.
- **Idempotency / dedup indexes**: duplicate backlink, double "Check now", replayed Stripe webhook,
  re-run crawl check row, pending invitation — [docs/07 §4](../../docs/07-job-queue.md).
- **`updated_at` trigger** fires on UPDATE.
- **Scheduler query plan**: `EXPLAIN ANALYZE` confirms an index scan on `backlinks_due_idx` and
  **no** sequential scan over a 3000-row table — [docs/03 §4.1](../../docs/03-database-schema.md),
  §11.
- **Job claim query**: `FOR UPDATE SKIP LOCKED` locks the highest-priority rows and leaves lower
  bands pending; uses `jobs_claim_idx` — [docs/07 §2](../../docs/07-job-queue.md).
- **Value at Risk** aggregate is grouped by currency, never summed across — §33.

## What it does NOT replace

This is a smoke test, not the real gate. PGlite is single-connection and single-user, so it cannot
exercise:

- Neon's transaction pooler (open item **R1** — the SQLx statement-cache question), or
- `FOR UPDATE SKIP LOCKED` under genuine concurrency (only one connection here), or
- SQLx's compile-time query verification.

Those belong to `crates/db/tests` against a real Postgres/Neon once the Rust toolchain is available
([docs/13](../../docs/13-roadmap.md), Phase 1 exit criteria). This test runs alongside them, not
instead of them.
