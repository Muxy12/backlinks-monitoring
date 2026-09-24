-- 0009 — CSV imports and their staging rows
-- See docs/03-database-schema.md §3.7, §38

CREATE TABLE imports (
    id              uuid PRIMARY KEY,
    organization_id uuid          NOT NULL REFERENCES organizations ON DELETE CASCADE,
    project_id      uuid          NOT NULL,
    created_by      uuid          REFERENCES users ON DELETE SET NULL,
    filename        text          NOT NULL,
    source_format   text          NOT NULL,
    status          import_status NOT NULL DEFAULT 'uploaded',
    column_mapping  jsonb,
    total_rows      int,
    valid_rows      int,
    imported_rows   int,
    duplicate_rows  int,
    error_rows      int,
    errors          jsonb,
    created_at      timestamptz   NOT NULL DEFAULT now(),
    completed_at    timestamptz,
    CONSTRAINT imports_source_format CHECK (source_format IN ('generic', 'ahrefs', 'semrush')),
    CONSTRAINT imports_counts_non_negative CHECK (
        coalesce(total_rows, 0)     >= 0 AND coalesce(valid_rows, 0)     >= 0 AND
        coalesce(imported_rows, 0)  >= 0 AND coalesce(duplicate_rows, 0) >= 0 AND
        coalesce(error_rows, 0)     >= 0
    ),
    CONSTRAINT imports_project_fk FOREIGN KEY (organization_id, project_id)
        REFERENCES projects (organization_id, id) ON DELETE CASCADE
);

CREATE INDEX imports_org_idx ON imports (organization_id, created_at DESC);

-- Staging. A REAL table, not a temp table: temp tables do not survive Neon's
-- transaction pooler (docs/01 §2.2), and the import spans multiple jobs anyway.
CREATE TABLE import_rows (
    id          uuid PRIMARY KEY,
    import_id   uuid  NOT NULL REFERENCES imports ON DELETE CASCADE,
    row_number  int   NOT NULL,
    raw         jsonb NOT NULL,
    parsed      jsonb,
    error       text,
    backlink_id uuid,
    -- Makes row processing idempotent on retry: a re-run skips rows that
    -- already carry a backlink_id (docs/07 §4).
    CONSTRAINT import_rows_number_uq UNIQUE (import_id, row_number),
    CONSTRAINT import_rows_number_positive CHECK (row_number > 0)
);

-- Chunked processing walks rows in order within an import (docs/07 §5).
CREATE INDEX import_rows_pending_idx ON import_rows (import_id, row_number)
    WHERE backlink_id IS NULL AND error IS NULL;
