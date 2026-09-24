-- 0004 — clients, projects, campaigns, publishers
-- See docs/03-database-schema.md §3.3
--
-- Every table here carries UNIQUE (organization_id, id). That looks redundant
-- next to the primary key, and it is — on purpose. It is the anchor for the
-- composite foreign keys that make a cross-tenant reference physically
-- impossible (docs/03 §1.1, docs/05 layer 3).

-- ─── Clients (§20) ──────────────────────────────────────────────────────────
CREATE TABLE clients (
    id              uuid PRIMARY KEY,
    organization_id uuid        NOT NULL REFERENCES organizations ON DELETE CASCADE,
    name            text        NOT NULL,
    company_name    text,
    website         text,
    contact_name    text,
    contact_email   citext,
    notes           text,
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT clients_name_not_blank CHECK (length(btrim(name)) > 0),
    CONSTRAINT clients_org_id_uq      UNIQUE (organization_id, id)
);

CREATE INDEX clients_org_idx ON clients (organization_id, name);

CREATE TRIGGER clients_set_updated_at
    BEFORE UPDATE ON clients
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- ─── Projects (§21) ─────────────────────────────────────────────────────────
CREATE TABLE projects (
    id                      uuid PRIMARY KEY,
    organization_id         uuid           NOT NULL REFERENCES organizations ON DELETE CASCADE,
    client_id               uuid,
    name                    text           NOT NULL,
    domain                  text           NOT NULL,
    default_check_frequency interval       NOT NULL DEFAULT '7 days',
    timezone                text           NOT NULL DEFAULT 'UTC',
    status                  project_status NOT NULL DEFAULT 'active',
    created_at              timestamptz    NOT NULL DEFAULT now(),
    updated_at              timestamptz    NOT NULL DEFAULT now(),
    archived_at             timestamptz,
    CONSTRAINT projects_name_not_blank CHECK (length(btrim(name)) > 0),
    CONSTRAINT projects_domain_not_blank CHECK (length(btrim(domain)) > 0),
    -- Guard against a frequency that would hammer publishers (docs/06 §5)
    -- or one so long it is indistinguishable from "off".
    CONSTRAINT projects_frequency_sane
        CHECK (default_check_frequency >= interval '1 hour'
           AND default_check_frequency <= interval '180 days'),
    CONSTRAINT projects_org_id_uq UNIQUE (organization_id, id),
    CONSTRAINT projects_client_fk FOREIGN KEY (organization_id, client_id)
        REFERENCES clients (organization_id, id) ON DELETE SET NULL
);

CREATE INDEX projects_org_idx    ON projects (organization_id, name);
CREATE INDEX projects_client_idx ON projects (client_id) WHERE client_id IS NOT NULL;

CREATE TRIGGER projects_set_updated_at
    BEFORE UPDATE ON projects
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- ─── Campaigns (§22) ────────────────────────────────────────────────────────
CREATE TABLE campaigns (
    id               uuid PRIMARY KEY,
    organization_id  uuid            NOT NULL REFERENCES organizations ON DELETE CASCADE,
    project_id       uuid            NOT NULL,
    name             text            NOT NULL,
    description      text,
    status           campaign_status NOT NULL DEFAULT 'draft',
    start_date       date,
    end_date         date,
    target_backlinks int,
    budget           numeric(14,2),
    currency         char(3),
    created_at       timestamptz     NOT NULL DEFAULT now(),
    updated_at       timestamptz     NOT NULL DEFAULT now(),
    CONSTRAINT campaigns_name_not_blank CHECK (length(btrim(name)) > 0),
    CONSTRAINT campaigns_dates      CHECK (end_date IS NULL OR start_date IS NULL OR end_date >= start_date),
    CONSTRAINT campaigns_target_pos CHECK (target_backlinks IS NULL OR target_backlinks > 0),
    CONSTRAINT campaigns_budget_pos CHECK (budget IS NULL OR budget >= 0),
    -- Money without a currency is not a number we can ever display (docs/03 §1).
    CONSTRAINT campaigns_budget_currency CHECK (budget IS NULL OR currency IS NOT NULL),
    CONSTRAINT campaigns_currency_format CHECK (currency IS NULL OR currency ~ '^[A-Z]{3}$'),
    CONSTRAINT campaigns_org_id_uq UNIQUE (organization_id, id),
    CONSTRAINT campaigns_project_fk FOREIGN KEY (organization_id, project_id)
        REFERENCES projects (organization_id, id) ON DELETE CASCADE
);

CREATE INDEX campaigns_org_idx     ON campaigns (organization_id, status);
CREATE INDEX campaigns_project_idx ON campaigns (project_id);

CREATE TRIGGER campaigns_set_updated_at
    BEFORE UPDATE ON campaigns
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- ─── Publishers (§31) ───────────────────────────────────────────────────────
CREATE TABLE publishers (
    id              uuid PRIMARY KEY,
    organization_id uuid        NOT NULL REFERENCES organizations ON DELETE CASCADE,
    name            text        NOT NULL,
    domain          text        NOT NULL,
    contact_name    text,
    contact_email   citext,
    notes           text,
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT publishers_name_not_blank   CHECK (length(btrim(name)) > 0),
    CONSTRAINT publishers_domain_not_blank CHECK (length(btrim(domain)) > 0),
    CONSTRAINT publishers_org_id_uq        UNIQUE (organization_id, id),
    -- One publisher record per domain per org, so the §31 rollups (spend,
    -- live/lost, average lifespan, removal rate) have a stable grouping key.
    CONSTRAINT publishers_org_domain_uq    UNIQUE (organization_id, domain)
);

CREATE TRIGGER publishers_set_updated_at
    BEFORE UPDATE ON publishers
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
