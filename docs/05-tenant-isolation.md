# 05 — Tenant isolation

**Spec refs:** §18, §45, §57.12. **Decision and rationale:** [ADR-0004](adr/0004-tenant-isolation-in-rust.md).

This document is the operational companion to ADR-0004, which explains *why* isolation is enforced
in Rust rather than by Postgres RLS (short version: Neon's transaction pooler does not support the
session variables an RLS scheme would depend on).

---

## 1. The four layers

| Layer | Mechanism | Catches |
|---|---|---|
| 1. Type | `OrgScope` has no public constructor; only `auth::authorize_org` mints one | A handler that never checked membership |
| 2. Query | Every repository fn takes `OrgScope` first and injects `WHERE organization_id = $1` | A query that forgets the tenant filter |
| 3. Schema | `organization_id NOT NULL` + composite FKs on `(organization_id, id)` | A *write* that crosses tenants, even via a bug in layers 1–2 |
| 4. Test | Table-driven suite over every route | A new route added without isolation |

Layer 3 is the one that does not depend on our code being correct. A backlink in org A referencing
a project in org B is rejected by the database itself
([03 §1.1](03-database-schema.md#11-the-composite-fk-pattern)).

---

## 2. Role matrix (§18)

| Capability | owner | admin | member | viewer |
|---|:--:|:--:|:--:|:--:|
| View everything in the org | ✓ | ✓ | ✓ | ✓ |
| Create/edit backlinks, campaigns, clients, projects | ✓ | ✓ | ✓ | |
| Trigger "Check now" | ✓ | ✓ | ✓ | |
| Import CSV | ✓ | ✓ | ✓ | |
| Delete backlinks / projects / clients | ✓ | ✓ | | |
| Invite / remove members | ✓ | ✓ | | |
| Change member roles | ✓ | ✓¹ | | |
| Manage billing & subscription | ✓ | | | |
| Delete the organization | ✓ | | | |
| Transfer ownership | ✓ | | | |

¹ An admin cannot promote to `owner` or modify an `owner`.

Enforced as `scope.require(Role::Admin)?` at the top of the handler, **after** `OrgScope` has been
minted. Scope answers "which org"; role answers "may they". Both are required; neither substitutes
for the other.

Invariant: **an organization always has at least one `owner`.** Removing or demoting the last owner
is rejected at the repository layer, not just in the UI — otherwise an org becomes permanently
unbillable and unadministrable.

---

## 3. IDOR prevention rules

1. Every by-id fetch is `WHERE id = $1 AND organization_id = $2`. Never fetch-then-compare in Rust —
   that pattern leaks existence through timing and through any path that forgets the comparison.
2. Cross-tenant and non-existent both return **404**. Never 403 for a resource in another org.
3. Nested resources validate the **whole chain**: a campaign under a project must confirm the
   project is in scope, not merely that the campaign is.
4. IDs in request *bodies* are validated against scope exactly like IDs in paths. Setting
   `campaign_id` on a backlink checks that the campaign belongs to the same org.
5. Sessions cache nothing about membership. Membership is re-read per request, so removing a user
   takes effect on their next request rather than at their next login.

---

## 4. Where tenancy deliberately does not apply

- **`jobs`** carries a nullable `organization_id` for attribution and observability only. The worker
  claims across all tenants — it is infrastructure, not a tenant actor.
- **The crawler** has no tenant concept at all. It receives URLs and returns results
  ([06](06-crawler.md)). This is what allows the §27 dedup to be safe: there is no tenant state in
  the component that does the sharing.
- **The public checker** (§50) is unauthenticated by design and touches no tenant data.
