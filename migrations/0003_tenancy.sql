-- 0003 — organizations, membership, invitations
-- See docs/03-database-schema.md §3.2, docs/05-tenant-isolation.md

CREATE TABLE organizations (
    id         uuid PRIMARY KEY,
    name       text        NOT NULL,
    slug       citext      NOT NULL UNIQUE,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT organizations_name_not_blank CHECK (length(btrim(name)) > 0),
    CONSTRAINT organizations_slug_format    CHECK (slug ~ '^[a-z0-9][a-z0-9-]{1,62}[a-z0-9]$')
);

CREATE TRIGGER organizations_set_updated_at
    BEFORE UPDATE ON organizations
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TABLE organization_members (
    organization_id uuid        NOT NULL REFERENCES organizations ON DELETE CASCADE,
    user_id         uuid        NOT NULL REFERENCES users ON DELETE CASCADE,
    role            org_role    NOT NULL,
    created_at      timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (organization_id, user_id)
);

-- The hot path: "which orgs is this user in", on every authenticated request.
-- The PK covers (organization_id, user_id); this covers the other direction.
CREATE INDEX org_members_user_idx ON organization_members (user_id);

-- docs/05 §2: an organization must always retain at least one owner.
-- The rule is enforced in the repository layer (it needs to distinguish
-- "demote" from "delete the org"), but this index makes the check cheap.
CREATE INDEX org_members_owner_idx ON organization_members (organization_id)
    WHERE role = 'owner';

CREATE TABLE organization_invitations (
    id              uuid PRIMARY KEY,
    organization_id uuid        NOT NULL REFERENCES organizations ON DELETE CASCADE,
    email           citext      NOT NULL,
    role            org_role    NOT NULL,
    invited_by      uuid        REFERENCES users ON DELETE SET NULL,
    token_hash      bytea       NOT NULL UNIQUE,
    expires_at      timestamptz NOT NULL,
    accepted_at     timestamptz,
    created_at      timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT org_invites_hash_len CHECK (octet_length(token_hash) = 32),
    -- An invitation cannot grant ownership; ownership transfer is its own flow.
    CONSTRAINT org_invites_role CHECK (role <> 'owner')
);

-- One outstanding invitation per email per org. Partial, so a re-invite after
-- acceptance is allowed (e.g. the member was removed and is being re-added).
CREATE UNIQUE INDEX org_invites_pending_uq
    ON organization_invitations (organization_id, email)
    WHERE accepted_at IS NULL;
