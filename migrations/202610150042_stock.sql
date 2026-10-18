CREATE TABLE stock_card(
 company_id bigint NOT NULL, catalog_id bigint NOT NULL,
 revision bigint NOT NULL DEFAULT 1, opened_on date NOT NULL,
 quantity numeric(18,4) NOT NULL DEFAULT 0,
 PRIMARY KEY(company_id,catalog_id),
 FOREIGN KEY(company_id,catalog_id) REFERENCES catalog(company_id,id),
 CHECK(abs(quantity)<=1000000000)
);
CREATE TABLE stock_movement(
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 company_id bigint NOT NULL, catalog_id bigint NOT NULL,
 quantity numeric(18,4) NOT NULL, booked_on date NOT NULL,
 kind text NOT NULL CHECK(kind IN('opening','manual','count','reversal','invoice','retail','received','credit')),
 reason text NOT NULL, item jsonb NOT NULL,
 reverses_id bigint, actor_id bigint NOT NULL REFERENCES app_user(id),
 created_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(company_id,id),UNIQUE(company_id,reverses_id),
 FOREIGN KEY(company_id,catalog_id) REFERENCES stock_card(company_id,catalog_id),
 FOREIGN KEY(company_id,reverses_id) REFERENCES stock_movement(company_id,id)
);
CREATE INDEX stock_history ON stock_movement(company_id,catalog_id,booked_on,id);
CREATE TABLE stock_command(
 company_id bigint NOT NULL REFERENCES company(id),request_id uuid NOT NULL,
 actor_id bigint NOT NULL REFERENCES app_user(id),input jsonb NOT NULL,result jsonb NOT NULL,
 created_at timestamptz NOT NULL DEFAULT now(),PRIMARY KEY(company_id,request_id)
);
