# 04 — Entity Relationship Diagram

**Spec ref:** §56.6. Field definitions live in [03-database-schema.md](03-database-schema.md).

## Tenant hierarchy (§18)

```
User ──┬── OrganizationMember ──┬── Organization
       │      (role)            │
       │                        ├── Client
       │                        │     └── Project
       │                        │           ├── Campaign
       │                        │           └── Backlink ──┬── BacklinkCheck
       │                        │                          └── BacklinkEvent
       │                        ├── Publisher ─────────────┘ (backlinks reference)
       │                        ├── Tag ── BacklinkTag ────┘
       │                        ├── Import ── ImportRow
       │                        ├── Subscription ── UsageRecord
       │                        ├── Notification
       │                        └── AuditLog
       └── Session / AuthToken / NotificationPreference
```

A user belongs to **many** organizations (§18). `organization_members` is the join, and it carries
the role — a user can be an `owner` of one org and a `viewer` of another.

## Full ERD

```mermaid
erDiagram
    USERS ||--o{ SESSIONS : "has"
    USERS ||--o{ AUTH_TOKENS : "has"
    USERS ||--o{ ORGANIZATION_MEMBERS : "member of"
    USERS ||--o{ NOTIFICATION_PREFERENCES : "configures"
    ORGANIZATIONS ||--o{ ORGANIZATION_MEMBERS : "has"
    ORGANIZATIONS ||--o{ ORGANIZATION_INVITATIONS : "issues"
    ORGANIZATIONS ||--o{ CLIENTS : "owns"
    ORGANIZATIONS ||--o{ PROJECTS : "owns"
    ORGANIZATIONS ||--o{ CAMPAIGNS : "owns"
    ORGANIZATIONS ||--o{ PUBLISHERS : "owns"
    ORGANIZATIONS ||--o{ BACKLINKS : "owns"
    ORGANIZATIONS ||--o{ TAGS : "owns"
    ORGANIZATIONS ||--o{ IMPORTS : "owns"
    ORGANIZATIONS ||--o{ NOTIFICATIONS : "receives"
    ORGANIZATIONS ||--o{ AUDIT_LOGS : "records"
    ORGANIZATIONS ||--|| SUBSCRIPTIONS : "billed by"
    ORGANIZATIONS ||--o{ USAGE_RECORDS : "meters"

    CLIENTS   ||--o{ PROJECTS  : "has"
    PROJECTS  ||--o{ CAMPAIGNS : "has"
    PROJECTS  ||--o{ BACKLINKS : "has"
    PROJECTS  ||--o{ IMPORTS   : "target of"
    CAMPAIGNS ||--o{ BACKLINKS : "groups"
    PUBLISHERS ||--o{ BACKLINKS : "supplies"

    BACKLINKS ||--o{ BACKLINK_CHECKS : "checked by"
    BACKLINKS ||--o{ BACKLINK_EVENTS : "changed by"
    BACKLINKS ||--o{ BACKLINK_TAGS   : "tagged"
    TAGS      ||--o{ BACKLINK_TAGS   : "applies to"
    BACKLINK_CHECKS ||--o{ BACKLINK_EVENTS : "produced"

    IMPORTS ||--o{ IMPORT_ROWS : "stages"
    SUBSCRIPTIONS ||--o{ STRIPE_EVENTS : "idempotency"

    USERS {
        uuid id PK
        citext email UK
        text password_hash
        timestamptz email_verified_at
    }
    ORGANIZATIONS {
        uuid id PK
        text name
        citext slug UK
    }
    ORGANIZATION_MEMBERS {
        uuid organization_id PK_FK
        uuid user_id PK_FK
        org_role role
    }
    CLIENTS {
        uuid id PK
        uuid organization_id FK
        text name
        text website
    }
    PROJECTS {
        uuid id PK
        uuid organization_id FK
        uuid client_id FK
        text domain
        interval default_check_frequency
    }
    CAMPAIGNS {
        uuid id PK
        uuid organization_id FK
        uuid project_id FK
        campaign_status status
        numeric budget
        int target_backlinks
    }
    PUBLISHERS {
        uuid id PK
        uuid organization_id FK
        text domain UK
    }
    BACKLINKS {
        uuid id PK
        uuid organization_id FK
        uuid project_id FK
        uuid campaign_id FK
        uuid publisher_id FK
        text source_url
        text source_url_normalized
        text target_url
        rel_type expected_rel_type
        backlink_status status
        backlink_status previous_status
        numeric cost
        warranty_type warranty_type
        date warranty_expiration
        timestamptz next_check_at
        int consecutive_failure_count
    }
    BACKLINK_CHECKS {
        uuid id PK
        uuid backlink_id FK
        uuid crawl_id
        boolean found
        backlink_status status_after
        jsonb raw
    }
    BACKLINK_EVENTS {
        uuid id PK
        uuid backlink_id FK
        uuid check_id FK
        event_type event_type
        jsonb previous_data
        jsonb current_data
    }
    JOBS {
        uuid id PK
        uuid organization_id FK
        text job_type
        jsonb payload
        job_status status
        int priority
        int attempts
        timestamptz scheduled_at
        timestamptz locked_at
        text locked_by
    }
```

`JOBS` is drawn unattached on purpose: it references `organization_id` for observability and quota
attribution, but it is infrastructure, not part of the domain graph. A job referencing a deleted
organization is tolerable; a backlink referencing a deleted project is not.

## Cardinality notes

- `Organization` → `Subscription` is **1:1**. An org has exactly one billing relationship.
- `Backlink` → `Campaign` and `Backlink` → `Publisher` are **optional**. A link can exist before it
  is attributed to either, which is what CSV import produces on day one.
- `BacklinkCheck` → `BacklinkEvent` is 1:many and optional — most checks produce **no** event. That
  is the normal, healthy case, and it is why events are a separate table rather than a flag on
  checks.
- `Project` → `Client` is optional, so a solo consultant (§2) never has to invent a client record
  to get started.
