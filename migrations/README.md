# Migrations

SQLx migrations for Neon PostgreSQL. Forward-only, numbered, reviewed.
Schema design: [docs/03-database-schema.md](../docs/03-database-schema.md).

| File | Contents |
|---|---|
| `0001_extensions_enums.sql` | `citext`, all 10 enum types, the shared `set_updated_at()` trigger fn |
| `0002_identity.sql` | `users`, `sessions`, `auth_tokens` |
| `0003_tenancy.sql` | `organizations`, `organization_members`, `organization_invitations` |
| `0004_operations.sql` | `clients`, `projects`, `campaigns`, `publishers` |
| `0005_backlinks.sql` | `backlinks` + all list/scheduler/attention indexes |
| `0006_history.sql` | `backlink_checks`, `backlink_events` |
| `0007_tags.sql` | `tags`, `backlink_tags` |
| `0008_jobs.sql` | `jobs` (the durable queue) |
| `0009_imports.sql` | `imports`, `import_rows` |
| `0010_billing.sql` | `subscriptions`, `stripe_events`, `usage_records` |
| `0011_notifications.sql` | `notifications`, `notification_preferences` |
| `0012_audit_logs.sql` | `audit_logs` |

## Running

Against a real database, once the Rust toolchain is available
([docs/10 §4](../docs/10-environments.md)):

```bash
# Local (Docker Postgres or a personal Neon branch)
cargo sqlx migrate run

# Production — direct URL, NEVER the pooler, gated CI step only (docs/adr/0005, §42)
DATABASE_URL="$DATABASE_DIRECT_URL" sqlx migrate run
```

## Verifying without the toolchain

Every migration here has been applied and exercised in **PGlite** (Postgres 17, WASM) — see
[`tools/schema-check`](../tools/schema-check). That harness caught a real NULL three-valued-logic
bug in the warranty CHECK in `0005` (`x > 0` is NULL, not false, when `x` is NULL, and a NULL CHECK
is treated as satisfied). It is a smoke test, not a replacement for `crates/db/tests` against real
Neon — notably it cannot cover the pooler statement-cache question (open item R1) or true
concurrency on the job claim.

## Conventions

- UUID **v7** primary keys, generated app-side (docs/03 §1).
- `timestamptz` everywhere, never `timestamp`.
- Money is `numeric(14,2)` + a `char(3)` currency, guarded so cost cannot exist without a currency.
- Every tenant-owned parent carries `UNIQUE (organization_id, id)` to anchor composite FKs — this is
  tenant-isolation layer 3 (docs/05), and it is enforced by the database, not by Rust.
- Forward-only. Destructive changes split across two deploys (add + backfill, then drop) so a
  rollback never loses data (docs/adr/0005).
