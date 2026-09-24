-- 0002 — users, sessions, auth tokens
-- See docs/03-database-schema.md §3.1, docs/11-security-risks.md S4

CREATE TABLE users (
    id                 uuid PRIMARY KEY,
    email              citext      NOT NULL UNIQUE,
    password_hash      text        NOT NULL,           -- Argon2id PHC string
    name               text        NOT NULL,
    email_verified_at  timestamptz,
    last_login_at      timestamptz,
    failed_login_count int         NOT NULL DEFAULT 0,
    locked_until       timestamptz,
    created_at         timestamptz NOT NULL DEFAULT now(),
    updated_at         timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT users_name_not_blank CHECK (length(btrim(name)) > 0)
);

CREATE TRIGGER users_set_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- Opaque session tokens. We store a SHA-256 of the token, never the token
-- itself: a database dump must not be a set of working credentials (S4).
CREATE TABLE sessions (
    id              uuid PRIMARY KEY,
    user_id         uuid        NOT NULL REFERENCES users ON DELETE CASCADE,
    token_hash      bytea       NOT NULL UNIQUE,
    csrf_token_hash bytea       NOT NULL,
    user_agent      text,
    ip_address      inet,
    expires_at      timestamptz NOT NULL,
    revoked_at      timestamptz,
    created_at      timestamptz NOT NULL DEFAULT now(),
    last_seen_at    timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT sessions_token_hash_len  CHECK (octet_length(token_hash) = 32),
    CONSTRAINT sessions_csrf_hash_len   CHECK (octet_length(csrf_token_hash) = 32)
);

-- Partial: only live sessions are ever looked up by user.
CREATE INDEX sessions_user_idx   ON sessions (user_id)    WHERE revoked_at IS NULL;
CREATE INDEX sessions_expiry_idx ON sessions (expires_at) WHERE revoked_at IS NULL;

-- Single-use, hashed, short-lived. One shape for both flows, discriminated
-- by `purpose`, so expiry and consumption logic is written once.
CREATE TABLE auth_tokens (
    id          uuid PRIMARY KEY,
    user_id     uuid        NOT NULL REFERENCES users ON DELETE CASCADE,
    purpose     text        NOT NULL,
    token_hash  bytea       NOT NULL UNIQUE,
    expires_at  timestamptz NOT NULL,
    consumed_at timestamptz,
    created_at  timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT auth_tokens_purpose CHECK (purpose IN ('email_verification', 'password_reset')),
    CONSTRAINT auth_tokens_hash_len CHECK (octet_length(token_hash) = 32)
);

CREATE INDEX auth_tokens_user_idx ON auth_tokens (user_id, purpose) WHERE consumed_at IS NULL;
