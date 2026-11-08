-- Spot order tracking ("Suivi de commande"): a lightweight per-spot purchase
-- tracker so the user can note what they ordered where, for how much, and how
-- far along it is. Not a payment integration — just bookkeeping that pairs with
-- the buy→monitor bridge.

CREATE TABLE IF NOT EXISTS spot_orders (
    id              uuid PRIMARY KEY,
    organization_id uuid NOT NULL REFERENCES organizations ON DELETE CASCADE,
    project_id      uuid NOT NULL,
    spot_id         uuid NOT NULL REFERENCES spots (id) ON DELETE CASCADE,
    status          text NOT NULL DEFAULT 'todo',   -- todo | in_progress | done | problem
    platform        text,                            -- marketplace / where it's ordered
    target_url      text,                            -- the page the backlink/citation points to
    price_cents     bigint,                          -- price paid, in cents
    currency        text,
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT spot_orders_status_valid
        CHECK (status IN ('todo', 'in_progress', 'done', 'problem')),
    CONSTRAINT spot_orders_project_fk FOREIGN KEY (organization_id, project_id)
        REFERENCES projects (organization_id, id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS spot_orders_spot_idx ON spot_orders (spot_id);
CREATE INDEX IF NOT EXISTS spot_orders_project_idx ON spot_orders (project_id);
