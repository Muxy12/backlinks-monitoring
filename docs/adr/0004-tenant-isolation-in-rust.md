# ADR-0004 — Tenant isolation enforced in Rust, not Postgres RLS

- **Status:** Accepted
- **Date:** 2026-09-24
- **Spec refs:** §18, §45, §57.12

## Context

Every tenant-owned query must be scoped to `organization_id`, and a user must never reach another
organization's data by guessing an ID (IDOR). Two standard approaches:

1. **Postgres Row-Level Security**, with the current org pushed into a session GUC
   (`SET LOCAL app.org_id = ...`) and policies reading `current_setting('app.org_id')`.
2. **Application-layer enforcement**: the type system makes an unscoped query impossible to write.

## Decision

Approach 2 — enforcement in Rust — with a type-level guard, not a convention.

RLS is rejected for a concrete reason, not a stylistic one: **Neon's pooler does not support session
variables** (`SET`/`RESET`) ([01 §2.2](../01-research-findings.md#22-features-that-do-not-work-through-the-neon-pooler)).
In transaction pooling, the connection carrying our `SET LOCAL` is not guaranteed to be the
connection carrying the next statement. An RLS scheme built on a session GUC over a transaction-mode
pooler is not "defense in depth," it is a policy that silently fails open or errors under load.
`SET LOCAL` inside an explicit transaction would work, but it forces every read onto an explicit
transaction and still leaves the failure mode one refactor away.

### The mechanism

A tenant-scoped context is the only way to reach a repository:

```rust
/// Proof that the current actor is a member of `organization_id`.
/// Constructed ONLY by `auth::authorize_org`, which does a membership lookup.
/// No public constructor, no Deserialize, no From<Uuid>.
#[derive(Clone, Copy, Debug)]
pub struct OrgScope {
    organization_id: OrganizationId,
    role: Role,
}
```

- `OrgScope` cannot be deserialized from a request body or path parameter. The only way to get one
  is `auth::authorize_org(&pool, user_id, org_id) -> Result<OrgScope, AuthError>`, which performs
  the `organization_members` lookup. This is the §18 rule — *never trust `organization_id` from the
  frontend* — expressed as a type rather than a code review checklist item.
- Every repository function takes `scope: OrgScope` as its **first** argument and injects
  `WHERE organization_id = $1` itself. Callers cannot supply the value.
- Fetch-by-id is always `WHERE id = $1 AND organization_id = $2`. A cross-tenant id returns
  `NotFound`, never `Forbidden` — we do not confirm that someone else's row exists.
- Role checks (`owner` / `admin` / `member` / `viewer`) are a second gate on top of scope:
  `scope.require(Role::Admin)?`.

### Defence in depth that does not depend on the pooler

- **`NOT NULL` + FK on every `organization_id`** column, so orphaned or null-tenant rows cannot be
  written at all.
- **Composite FKs** where a child could otherwise drift across tenants. A `backlinks` row references
  `(organization_id, project_id)` against a `UNIQUE (organization_id, id)` on `projects`, so a
  backlink physically cannot point at another org's project. This is the important one: it makes the
  dangerous state unrepresentable in the database, with no session state involved.
- **IDs are UUIDv7**, not sequential integers — not a security control, but it removes trivial
  enumeration.

### Tests that must exist before this is believed

`apps/api/tests/tenant_isolation.rs` runs, for every tenant-owned resource:

- org A creates a resource; org B `GET`/`PATCH`/`DELETE`s it by id → **404**, on every route.
- org B lists → A's rows are absent.
- a `viewer` attempting a write → **403**.
- a user removed from an org loses access on the next request (no stale session cache).

The test is table-driven over the route list so that adding a route without an isolation test fails
the suite.

## Consequences

- Isolation is a compile-time property of the repository API, so the common failure (a new query
  that forgets the `WHERE`) becomes a type error.
- We give up the database-level backstop RLS would provide against a bug in the Rust layer. The
  composite-FK constraints recover part of that, and the table-driven isolation suite covers the
  rest.
- If we later move off a transaction-mode pooler, RLS can be added underneath this without changing
  a single call site.
