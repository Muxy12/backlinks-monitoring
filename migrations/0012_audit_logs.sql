-- 0012 — audit log
-- See docs/03-database-schema.md §3.8, §45

CREATE TABLE audit_logs (
    id              uuid PRIMARY KEY,
    organization_id uuid REFERENCES organizations ON DELETE CASCADE,
    -- Nullable so the trail survives the actor being deleted. An audit log
    -- that disappears with the account it incriminates is not an audit log.
    actor_user_id   uuid REFERENCES users ON DELETE SET NULL,
    actor_email     citext,          -- denormalized snapshot, for the same reason
    action          text NOT NULL,   -- e.g. 'backlink.delete', 'member.role_change'
    resource_type   text,
    resource_id     uuid,
    before          jsonb,
    after           jsonb,
    ip_address      inet,
    user_agent      text,
    created_at      timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT audit_logs_action_not_blank CHECK (length(btrim(action)) > 0)
);

CREATE INDEX audit_logs_org_idx      ON audit_logs (organization_id, created_at DESC);
CREATE INDEX audit_logs_actor_idx    ON audit_logs (actor_user_id, created_at DESC)
    WHERE actor_user_id IS NOT NULL;
CREATE INDEX audit_logs_resource_idx ON audit_logs (resource_type, resource_id)
    WHERE resource_id IS NOT NULL;
