CREATE TABLE cash_settings (
 company_id bigint PRIMARY KEY REFERENCES company(id),
 account_id bigint, revision bigint NOT NULL DEFAULT 1,
 FOREIGN KEY(company_id,account_id) REFERENCES cash_account(company_id,id)
);
ALTER TABLE cash_movement ADD COLUMN automatic boolean NOT NULL DEFAULT false;
ALTER TABLE cash_movement ADD COLUMN issuer jsonb;
UPDATE cash_movement m SET issuer=c.settings FROM company c WHERE c.id=m.company_id;
ALTER TABLE cash_movement ALTER COLUMN issuer SET NOT NULL;
CREATE TABLE cash_pdf (
 company_id bigint NOT NULL, movement_id bigint NOT NULL, revision bigint NOT NULL,
 language text NOT NULL CHECK(language IN ('cs','en')), content bytea NOT NULL,
 PRIMARY KEY(company_id,movement_id,revision,language),
 FOREIGN KEY(company_id,movement_id) REFERENCES cash_movement(company_id,id)
);
