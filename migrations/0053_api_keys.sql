-- 0053 — AI-era refactor, Phase 2: subscriber API keys (MCP + keyed REST auth).
--
-- See docs/product/21 §4.2 and 22 §1. Per-workspace Bearer keys (`lp_live_…`)
-- for the MCP server and the future keyed REST API. The secret is shown once;
-- only its SHA-256 hash is stored (same scheme as sessions/tokens, S4), so a DB
-- dump holds nothing replayable. `prefix` is the non-secret display id, `scopes`
-- the granted permissions. Provider keys (DataForSEO …) are unrelated and never
-- stored here.

CREATE TABLE IF NOT EXISTS api_keys (
    id              uuid PRIMARY KEY,
    organization_id uuid   NOT NULL REFERENCES organizations ON DELETE CASCADE,
    name            text   NOT NULL,
    key_hash        bytea  NOT NULL,          -- SHA-256 of the full secret (32 bytes)
    prefix          text   NOT NULL,          -- public display id, e.g. 'lp_live_ab12cd'
    scopes          text[] NOT NULL DEFAULT '{}',
    last_used_at    timestamptz,
    created_at      timestamptz NOT NULL DEFAULT now(),
    revoked_at      timestamptz,

    CONSTRAINT api_keys_name_not_blank CHECK (length(btrim(name)) > 0),
    CONSTRAINT api_keys_hash_uq UNIQUE (key_hash)
);

-- Auth looks a key up by its hash among the live (non-revoked) keys.
CREATE INDEX IF NOT EXISTS api_keys_live_idx
    ON api_keys (key_hash) WHERE revoked_at IS NULL;
-- The settings UI lists an org's keys newest first.
CREATE INDEX IF NOT EXISTS api_keys_org_idx
    ON api_keys (organization_id, created_at DESC);
