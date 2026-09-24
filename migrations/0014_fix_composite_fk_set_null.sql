-- 0014 — restrict composite-FK SET NULL to the child column (PostgreSQL 15+)
--
-- The (organization_id, <child>) foreign keys were declared plain
-- `ON DELETE SET NULL`, which Postgres applies to EVERY column in the key —
-- including the NOT NULL `organization_id`. So deleting a referenced campaign,
-- publisher or client would raise a not-null violation instead of unassigning
-- the child. `ON DELETE SET NULL (<child>)` nulls only the child column, which
-- is the intended behaviour.
--
-- Postgres cannot alter a constraint's action in place, so each FK is dropped
-- and re-added. Safe: same columns, same reference, only the ON DELETE action
-- changes. Requires PostgreSQL 15+ (CI = 17, Neon = 18).

ALTER TABLE backlinks DROP CONSTRAINT backlinks_campaign_fk;
ALTER TABLE backlinks
    ADD CONSTRAINT backlinks_campaign_fk
        FOREIGN KEY (organization_id, campaign_id)
        REFERENCES campaigns (organization_id, id)
        ON DELETE SET NULL (campaign_id);

ALTER TABLE backlinks DROP CONSTRAINT backlinks_publisher_fk;
ALTER TABLE backlinks
    ADD CONSTRAINT backlinks_publisher_fk
        FOREIGN KEY (organization_id, publisher_id)
        REFERENCES publishers (organization_id, id)
        ON DELETE SET NULL (publisher_id);

ALTER TABLE projects DROP CONSTRAINT projects_client_fk;
ALTER TABLE projects
    ADD CONSTRAINT projects_client_fk
        FOREIGN KEY (organization_id, client_id)
        REFERENCES clients (organization_id, id)
        ON DELETE SET NULL (client_id);
