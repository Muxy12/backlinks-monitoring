-- 0033 feedback: in-app feature requests and bug reports from users.
-- A logged-in user submits a suggestion or a bug; the platform admin triages it
-- from the backoffice. user_id / organization_id are SET NULL on delete so the
-- feedback survives account or workspace deletion; user_email is snapshotted so
-- the admin list still shows who submitted it afterward.

CREATE TABLE feedback (
    id               uuid        PRIMARY KEY,
    created_at       timestamptz NOT NULL DEFAULT now(),
    updated_at       timestamptz NOT NULL DEFAULT now(),
    -- 'feature' (a suggestion) or 'bug' (a report).
    kind             text        NOT NULL CHECK (kind IN ('feature', 'bug')),
    title            text        NOT NULL CHECK (char_length(title) BETWEEN 1 AND 160),
    body             text        NOT NULL CHECK (char_length(body) BETWEEN 1 AND 4000),
    -- Where the user was when they submitted (helps triage bugs). Optional.
    page_url         text,
    -- Triage state, owned by the admin.
    status           text        NOT NULL DEFAULT 'open'
                      CHECK (status IN ('open', 'planned', 'in_progress', 'done', 'declined')),
    user_id          uuid        REFERENCES users(id) ON DELETE SET NULL,
    user_email       text        NOT NULL,
    organization_id  uuid        REFERENCES organizations(id) ON DELETE SET NULL
);

-- Admin list: newest first, filterable by triage state.
CREATE INDEX feedback_status_idx ON feedback (status, created_at DESC);
-- A user's own submissions.
CREATE INDEX feedback_user_idx ON feedback (user_id, created_at DESC);
