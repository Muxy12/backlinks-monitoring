-- 0001 — extensions, enum types, shared trigger function
-- See docs/03-database-schema.md

CREATE EXTENSION IF NOT EXISTS citext;

-- ─── Enum types (docs/03 §2) ────────────────────────────────────────────────
CREATE TYPE org_role          AS ENUM ('owner', 'admin', 'member', 'viewer');

CREATE TYPE project_status    AS ENUM ('active', 'paused', 'archived');

CREATE TYPE campaign_status   AS ENUM ('draft', 'active', 'paused', 'completed', 'archived');

-- docs/08 §2. `archived` is deliberately NOT a status: archival lives in
-- backlinks.archived_at so that an archived link keeps its last known state.
CREATE TYPE backlink_status   AS ENUM (
    'pending', 'live', 'lost', 'changed', 'nofollow',
    'redirected', 'source_error', 'target_error', 'blocked', 'unknown'
);

CREATE TYPE rel_type          AS ENUM ('dofollow', 'nofollow', 'sponsored', 'ugc');

CREATE TYPE warranty_type     AS ENUM ('none', 'days', 'months', 'lifetime');

CREATE TYPE job_status        AS ENUM ('pending', 'processing', 'completed', 'failed', 'dead');

CREATE TYPE import_status     AS ENUM (
    'uploaded', 'parsing', 'mapping', 'validating',
    'ready', 'importing', 'completed', 'failed'
);

CREATE TYPE event_type        AS ENUM (
    'backlink_found', 'backlink_lost', 'backlink_restored',
    'anchor_changed', 'target_changed',
    'dofollow_to_nofollow', 'nofollow_to_dofollow', 'rel_changed',
    'source_redirected', 'source_404', 'source_error',
    'target_redirected', 'target_404', 'target_error',
    'noindex_detected', 'canonical_changed'
);

CREATE TYPE subscription_plan AS ENUM ('starter', 'pro', 'agency', 'agency_plus');

-- ─── Shared trigger function ────────────────────────────────────────────────
-- One implementation, attached to every table carrying updated_at.
CREATE OR REPLACE FUNCTION set_updated_at() RETURNS trigger AS $$
BEGIN
    NEW.updated_at := now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
