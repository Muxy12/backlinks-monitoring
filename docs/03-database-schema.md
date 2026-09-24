# 03 — Neon PostgreSQL schema

**Spec refs:** §12, §20–§23, §31, §41, §42
**ERD:** [04-erd.md](04-erd.md)

---

## 1. Conventions

| Convention | Choice | Why |
|---|---|---|
| Primary keys | `UUID` (**v7**, generated in Rust) | Time-ordered so B-tree inserts stay local, unlike v4. Non-enumerable, unlike `bigserial`. Generated app-side so we can build object graphs before touching the database. |
| Timestamps | `timestamptz`, always | §41. A backlink checked from a worker in one region and read in another must not disagree about when. Never `timestamp`. |
| Money | `numeric(14,2)` + `currency char(3)` | Never floats. Currency stored per row — agencies buy links in EUR and bill in USD. |
| Soft delete | `archived_at timestamptz` | Only where history matters (`backlinks`, `projects`). Everything else deletes for real. |
| Enums | Postgres `ENUM` types | Cheap, self-documenting, and SQLx maps them to Rust enums with `#[derive(sqlx::Type)]`. Adding a value is `ALTER TYPE ... ADD VALUE`, which is non-blocking. |
| Tenant column | `organization_id NOT NULL` on every tenant-owned table | No nullable tenants, ever. See [ADR-0004](adr/0004-tenant-isolation-in-rust.md). |
| Audit columns | `created_at`, `updated_at` with a trigger | One `set_updated_at()` trigger function reused everywhere. |

### 1.1 The composite-FK pattern

Every parent tenant-owned table carries a redundant-looking constraint:

```sql
ALTER TABLE projects ADD CONSTRAINT projects_org_id_uq UNIQUE (organization_id, id);
```

so that children can reference the pair:

```sql
ALTER TABLE backlinks ADD CONSTRAINT backlinks_project_fk
    FOREIGN KEY (organization_id, project_id) REFERENCES projects (organization_id, id);
```

This makes "a backlink in org A pointing at a project in org B" **unrepresentable**, enforced by the
database, with no session state involved — which matters because Neon's pooler rules out the
session-GUC approach ([01 §2.2](01-research-findings.md#22-features-that-do-not-work-through-the-neon-pooler)).
Applied to `projects`, `clients`, `campaigns`, `publishers`, `backlinks`, `tags`.

---

## 2. Enum types

```sql
CREATE TYPE org_role          AS ENUM ('owner','admin','member','viewer');
CREATE TYPE project_status    AS ENUM ('active','paused','archived');
CREATE TYPE campaign_status   AS ENUM ('draft','active','paused','completed','archived');
CREATE TYPE backlink_status   AS ENUM ('pending','live','lost','changed','nofollow',
                                       'redirected','source_error','target_error',
                                       'blocked','unknown');
CREATE TYPE rel_type          AS ENUM ('dofollow','nofollow','sponsored','ugc');
CREATE TYPE warranty_type     AS ENUM ('none','days','months','lifetime');
CREATE TYPE job_status        AS ENUM ('pending','processing','completed','failed','dead');
CREATE TYPE import_status     AS ENUM ('uploaded','parsing','mapping','validating',
                                       'ready','importing','completed','failed');
CREATE TYPE event_type        AS ENUM ('backlink_found','backlink_lost','backlink_restored',
                                       'anchor_changed','target_changed',
                                       'dofollow_to_nofollow','nofollow_to_dofollow','rel_changed',
                                       'source_redirected','source_404','source_error',
                                       'target_redirected','target_404','target_error',
                                       'noindex_detected','canonical_changed');
CREATE TYPE subscription_plan AS ENUM ('starter','pro','agency','agency_plus');
```

---

## 3. Core tables

### 3.1 Identity

```sql
CREATE TABLE users (
    id                   uuid PRIMARY KEY,
    email                citext NOT NULL UNIQUE,
    password_hash        text   NOT NULL,          -- Argon2id PHC string
    name                 text   NOT NULL,
    email_verified_at    timestamptz,
    last_login_at        timestamptz,
    failed_login_count   int    NOT NULL DEFAULT 0,
    locked_until         timestamptz,
    created_at           timestamptz NOT NULL DEFAULT now(),
    updated_at           timestamptz NOT NULL DEFAULT now()
);

-- Opaque session tokens. We store a SHA-256 of the token, never the token itself:
-- a database dump must not be a set of working credentials.
CREATE TABLE sessions (
    id             uuid PRIMARY KEY,
    user_id        uuid NOT NULL REFERENCES users ON DELETE CASCADE,
    token_hash     bytea NOT NULL UNIQUE,
    csrf_token_hash bytea NOT NULL,
    user_agent     text,
    ip_address     inet,
    expires_at     timestamptz NOT NULL,
    revoked_at     timestamptz,
    created_at     timestamptz NOT NULL DEFAULT now(),
    last_seen_at   timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX sessions_user_idx   ON sessions (user_id) WHERE revoked_at IS NULL;
CREATE INDEX sessions_expiry_idx ON sessions (expires_at) WHERE revoked_at IS NULL;

-- Single-use, hashed, short-lived. Same table shape for both flows, discriminated by `purpose`.
CREATE TABLE auth_tokens (
    id         uuid PRIMARY KEY,
    user_id    uuid NOT NULL REFERENCES users ON DELETE CASCADE,
    purpose    text NOT NULL CHECK (purpose IN ('email_verification','password_reset')),
    token_hash bytea NOT NULL UNIQUE,
    expires_at timestamptz NOT NULL,
    consumed_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now()
);
```

### 3.2 Tenancy

```sql
CREATE TABLE organizations (
    id          uuid PRIMARY KEY,
    name        text NOT NULL,
    slug        citext NOT NULL UNIQUE,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE organization_members (
    organization_id uuid NOT NULL REFERENCES organizations ON DELETE CASCADE,
    user_id         uuid NOT NULL REFERENCES users ON DELETE CASCADE,
    role            org_role NOT NULL,
    created_at      timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (organization_id, user_id)
);
-- The hot path: "which orgs is this user in" on every authenticated request.
CREATE INDEX org_members_user_idx ON organization_members (user_id);

CREATE TABLE organization_invitations (
    id              uuid PRIMARY KEY,
    organization_id uuid NOT NULL REFERENCES organizations ON DELETE CASCADE,
    email           citext NOT NULL,
    role            org_role NOT NULL,
    invited_by      uuid REFERENCES users ON DELETE SET NULL,
    token_hash      bytea NOT NULL UNIQUE,
    expires_at      timestamptz NOT NULL,
    accepted_at     timestamptz,
    created_at      timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX org_invites_pending_uq
    ON organization_invitations (organization_id, email) WHERE accepted_at IS NULL;
```

### 3.3 Operational hierarchy

`clients`, `projects`, `campaigns`, `publishers` follow §20–§22 and §31 field-for-field. The
structurally interesting parts:

```sql
CREATE TABLE projects (
    id                      uuid PRIMARY KEY,
    organization_id         uuid NOT NULL REFERENCES organizations ON DELETE CASCADE,
    client_id               uuid,
    name                    text NOT NULL,
    domain                  text NOT NULL,
    default_check_frequency interval NOT NULL DEFAULT '7 days',
    timezone                text NOT NULL DEFAULT 'UTC',
    status                  project_status NOT NULL DEFAULT 'active',
    created_at              timestamptz NOT NULL DEFAULT now(),
    updated_at              timestamptz NOT NULL DEFAULT now(),
    archived_at             timestamptz,
    UNIQUE (organization_id, id),                                    -- composite-FK anchor
    FOREIGN KEY (organization_id, client_id)
        REFERENCES clients (organization_id, id) ON DELETE SET NULL
);
```

`publishers` additionally carries a `UNIQUE (organization_id, domain)` — one publisher record per
domain per org, so the §31 rollups (spend, live/lost, average lifespan, removal rate) have a stable
grouping key. Those rollups are **computed on read**, not denormalized; at MVP row counts an
aggregate over an org's backlinks is a few milliseconds, and a stale counter is worse than a fast
query.

### 3.4 `backlinks` — the core entity

```sql
CREATE TABLE backlinks (
    id                        uuid PRIMARY KEY,
    organization_id           uuid NOT NULL REFERENCES organizations ON DELETE CASCADE,
    project_id                uuid NOT NULL,
    campaign_id               uuid,
    publisher_id              uuid,

    -- Source. The normalized columns are generated so they cannot drift from the raw value.
    source_url                text NOT NULL,
    source_url_normalized     text NOT NULL,   -- written by the app, see note below
    source_domain             text NOT NULL,   -- registrable domain (public suffix aware)

    -- Target
    target_url                text NOT NULL,
    target_url_normalized     text NOT NULL,
    target_domain             text NOT NULL,

    -- What the customer bought
    expected_anchor_text      text,
    expected_rel_type         rel_type NOT NULL DEFAULT 'dofollow',

    -- What we last actually saw
    detected_anchor_text      text,
    detected_target_url       text,
    detected_rel_attributes   text[],
    source_http_status        int,
    target_http_status        int,
    source_canonical          text,
    source_robots_meta        text,

    status                    backlink_status NOT NULL DEFAULT 'pending',
    previous_status           backlink_status,

    -- Commercial (§32, §33)
    cost                      numeric(14,2),
    currency                  char(3),
    purchase_date             date,
    warranty_type             warranty_type NOT NULL DEFAULT 'none',
    warranty_value            int,                 -- days or months, per warranty_type
    warranty_expiration       date,                -- materialized on write for indexable queries

    -- Monitoring
    first_checked_at          timestamptz,
    last_checked_at           timestamptz,
    next_check_at             timestamptz,
    last_seen_at              timestamptz,
    failure_count             int NOT NULL DEFAULT 0,
    consecutive_failure_count int NOT NULL DEFAULT 0,

    notes                     text,
    created_at                timestamptz NOT NULL DEFAULT now(),
    updated_at                timestamptz NOT NULL DEFAULT now(),
    archived_at               timestamptz,

    UNIQUE (organization_id, id),
    FOREIGN KEY (organization_id, project_id)   REFERENCES projects   (organization_id, id) ON DELETE CASCADE,
    FOREIGN KEY (organization_id, campaign_id)  REFERENCES campaigns  (organization_id, id) ON DELETE SET NULL,
    FOREIGN KEY (organization_id, publisher_id) REFERENCES publishers (organization_id, id) ON DELETE SET NULL,
    CHECK (cost IS NULL OR currency IS NOT NULL),
    CHECK (warranty_type NOT IN ('days','months') OR warranty_value IS NOT NULL)
);

-- Duplicate prevention (§38 "detect duplicates"): the same link, twice, in one project.
CREATE UNIQUE INDEX backlinks_dedupe_uq
    ON backlinks (project_id, source_url_normalized, target_url_normalized)
    WHERE archived_at IS NULL;
```

**On normalization columns:** `source_url_normalized` is produced by the same Rust normalizer the
crawler uses ([06 §3.1](06-crawler.md#31-normalization)) and written on insert/update. It is *not* a
Postgres generated column, because the rule (public suffix list, tracking-param stripping, IDN) is
not expressible in SQL without duplicating logic that would then drift. A database `CHECK` cannot
enforce it, so a Phase 7 consistency job re-derives and reports mismatches.

### 3.5 History

```sql
-- Every check, append-only. This is the raw record.
CREATE TABLE backlink_checks (
    id              uuid PRIMARY KEY,
    backlink_id     uuid NOT NULL,
    organization_id uuid NOT NULL,
    crawl_id        uuid NOT NULL,          -- shared by every backlink from one deduped fetch
    checked_at      timestamptz NOT NULL DEFAULT now(),
    found           boolean NOT NULL,
    status_after    backlink_status NOT NULL,
    source_status   int,
    target_status   int,
    anchor_text     text,
    rel_attributes  text[],
    is_dofollow     boolean,
    final_url       text,
    redirect_chain  text[],
    duration_ms     int,
    bytes           int,
    error_kind      text,
    raw             jsonb,                  -- full CrawlResult, for debugging
    FOREIGN KEY (organization_id, backlink_id) REFERENCES backlinks (organization_id, id) ON DELETE CASCADE
);
CREATE INDEX backlink_checks_bl_idx ON backlink_checks (backlink_id, checked_at DESC);
CREATE INDEX backlink_checks_crawl_idx ON backlink_checks (crawl_id);

-- Meaningful changes only. This is what the §30 timeline renders.
CREATE TABLE backlink_events (
    id              uuid PRIMARY KEY,
    backlink_id     uuid NOT NULL,
    organization_id uuid NOT NULL,
    check_id        uuid REFERENCES backlink_checks ON DELETE SET NULL,
    event_type      event_type NOT NULL,
    previous_data   jsonb,
    current_data    jsonb,
    detected_at     timestamptz NOT NULL DEFAULT now(),
    FOREIGN KEY (organization_id, backlink_id) REFERENCES backlinks (organization_id, id) ON DELETE CASCADE
);
CREATE INDEX backlink_events_bl_idx  ON backlink_events (backlink_id, detected_at DESC);
CREATE INDEX backlink_events_org_idx ON backlink_events (organization_id, detected_at DESC);
```

`backlink_checks` grows fastest — one row per backlink per cycle. Retention: full rows for 90 days,
then downsample to one row per backlink per day. `backlink_events` is small and kept **forever**;
it is the customer-visible history and §57.8 says every backlink has one. Retention is implemented
as a job, not a manual cleanup.

### 3.6 Jobs

Per [ADR-0003](adr/0003-postgres-job-queue.md) and §10. Full definition in
[07-job-queue.md](07-job-queue.md).

### 3.7 Imports

```sql
CREATE TABLE imports (
    id              uuid PRIMARY KEY,
    organization_id uuid NOT NULL REFERENCES organizations ON DELETE CASCADE,
    project_id      uuid NOT NULL,
    created_by      uuid REFERENCES users ON DELETE SET NULL,
    filename        text NOT NULL,
    source_format   text NOT NULL,       -- generic | ahrefs | semrush
    status          import_status NOT NULL DEFAULT 'uploaded',
    column_mapping  jsonb,
    total_rows      int,
    valid_rows      int,
    imported_rows   int,
    duplicate_rows  int,
    error_rows      int,
    errors          jsonb,
    created_at      timestamptz NOT NULL DEFAULT now(),
    completed_at    timestamptz
);

-- Staging. A real table, not a temp table: temp tables do not survive Neon's transaction pooler,
-- and the import spans multiple jobs anyway.
CREATE TABLE import_rows (
    id          uuid PRIMARY KEY,
    import_id   uuid NOT NULL REFERENCES imports ON DELETE CASCADE,
    row_number  int NOT NULL,
    raw         jsonb NOT NULL,
    parsed      jsonb,
    error       text,
    backlink_id uuid,
    UNIQUE (import_id, row_number)      -- makes row processing idempotent on retry
);
```

### 3.8 Billing, notifications, audit

`subscriptions` (Stripe customer/subscription ids, `plan`, `status`, period bounds,
`cancel_at_period_end`), `usage_records` (per-org monthly backlink counts for §39 limits),
`notifications` (in-app feed), `notification_preferences` (per-user, per-event-type, per-channel),
`audit_logs` (actor, action, resource, `before`/`after` JSONB, IP).

`subscriptions` carries `UNIQUE (stripe_subscription_id)` and a
`stripe_events (event_id PRIMARY KEY, processed_at)` companion table — that table *is* the webhook
idempotency mechanism required by §39. An insert conflict means "already handled, return 200."

`tags` + `backlink_tags` are a plain many-to-many with `UNIQUE (organization_id, name)` on tags.

---

## 4. Indexes

Per §12, plus the ones the actual query plans need. **None of these ship without `EXPLAIN ANALYZE`
against seeded data** (§12: "do not create indexes blindly").

### 4.1 The scheduler index — the most important one in the system

```sql
CREATE INDEX backlinks_due_idx
    ON backlinks (next_check_at)
    WHERE archived_at IS NULL AND next_check_at IS NOT NULL;
```

Partial, so it contains only rows that can ever be due. Archived backlinks and links with no
schedule are physically absent from the index, which keeps it small as the table grows into the
millions. The scheduler query (§11) is an index range scan with a `LIMIT`, never a table scan —
that is the §11 requirement that we never load millions of due backlinks into memory.

### 4.2 Tenant-scoped list/filter indexes

```sql
CREATE INDEX backlinks_org_status_idx   ON backlinks (organization_id, status);
CREATE INDEX backlinks_org_project_idx  ON backlinks (organization_id, project_id);
CREATE INDEX backlinks_org_created_idx  ON backlinks (organization_id, created_at DESC);
CREATE INDEX backlinks_project_status_idx ON backlinks (project_id, status);
CREATE INDEX backlinks_campaign_idx     ON backlinks (campaign_id) WHERE campaign_id IS NOT NULL;
CREATE INDEX backlinks_publisher_idx    ON backlinks (publisher_id) WHERE publisher_id IS NOT NULL;
CREATE INDEX backlinks_source_domain_idx ON backlinks (source_domain);
CREATE INDEX backlinks_source_norm_idx  ON backlinks (source_url_normalized);   -- dedup grouping
```

Every list index leads with `organization_id` because every tenant query filters on it first
(ADR-0004). An index that does not lead with the tenant column is useless to us.

### 4.3 Attention and warranty

```sql
-- Drives the §34 dashboard "needs attention" panel and §33 Value at Risk.
CREATE INDEX backlinks_attention_idx
    ON backlinks (organization_id, status)
    WHERE archived_at IS NULL
      AND status IN ('lost','changed','nofollow','source_error','target_error','redirected');

CREATE INDEX backlinks_warranty_idx
    ON backlinks (organization_id, warranty_expiration)
    WHERE warranty_expiration IS NOT NULL AND archived_at IS NULL;
```

Value at Risk (§33) is then a single indexed aggregate:

```sql
SELECT coalesce(sum(cost), 0), currency
FROM backlinks
WHERE organization_id = $1 AND archived_at IS NULL
  AND status IN ('lost','changed','nofollow','source_error','target_error','redirected')
GROUP BY currency;
```

Grouped by currency deliberately — summing EUR and USD into one number would be a lie, and §33 is
already careful that this figure is *recorded acquisition cost associated with links needing
attention*, not a claim of financial loss. The UI labels it that way.

---

## 5. Migrations

SQLx migrations in `/migrations`, forward-only, numbered, reviewed. Rules in
[ADR-0005](adr/0005-neon-connection-strategy.md#migrations) and [10](10-environments.md):

- Applied against `DATABASE_DIRECT_URL`, never the pooler.
- An explicit, gated CI step against production. **Never from application boot. Never from a preview
  deployment** (§42, §44).
- Destructive changes split across two deploys (add + backfill, then drop) so rollback never loses
  data.
- `sqlx migrate info` runs in CI on every PR against a throwaway branch to prove migrations apply
  cleanly from empty.
