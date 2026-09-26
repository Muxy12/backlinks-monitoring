-- 0022 — platform (super) admin flag on users.
--
-- Distinct from ORG roles (owner/admin/member/viewer in organization_members),
-- which grant power *inside one tenant*. `is_platform_admin` is a cross-tenant
-- capability: the operator who pilots the whole SaaS (see all orgs, all users,
-- grant/revoke platform admin). It is deliberately narrow and off by default;
-- the admin backoffice routes are the ONLY sanctioned cross-tenant surface, and
-- they check this flag on every call. Bootstrapped from the ADMIN_EMAILS env at
-- boot (no credentials committed) — the operator signs up normally, then the
-- matching account is promoted.

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS is_platform_admin boolean NOT NULL DEFAULT false;

-- Tiny partial index: the admin set is a handful of rows, looked up by the flag.
CREATE INDEX IF NOT EXISTS users_platform_admin_idx
    ON users (id)
    WHERE is_platform_admin;
