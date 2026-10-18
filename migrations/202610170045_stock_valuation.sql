CREATE TABLE stock_valuation (
 company_id bigint NOT NULL, catalog_id bigint NOT NULL, revision bigint NOT NULL,
 effective_on date NOT NULL, unit_cost numeric(18,4), reason text NOT NULL,
 actor_id bigint NOT NULL REFERENCES app_user(id), request_id uuid NOT NULL,
 input jsonb NOT NULL, created_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(company_id,catalog_id,revision), UNIQUE(company_id,request_id),
 FOREIGN KEY(company_id,catalog_id) REFERENCES stock_card(company_id,catalog_id),
 CHECK(unit_cost IS NULL OR (unit_cost>=0 AND unit_cost<=1000000000)),
 CHECK(revision>0)
);
CREATE INDEX stock_valuation_date ON stock_valuation(company_id,catalog_id,effective_on,revision);
