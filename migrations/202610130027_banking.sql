CREATE TABLE bank_account (
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 company_id bigint NOT NULL REFERENCES company(id), revision bigint NOT NULL DEFAULT 1,
 name text NOT NULL, iban text NOT NULL, currency text NOT NULL CHECK(currency='CZK'),
 archived boolean NOT NULL DEFAULT false, UNIQUE(company_id,id), UNIQUE(company_id,iban)
);
CREATE TABLE bank_series (
 company_id bigint NOT NULL REFERENCES company(id), year integer NOT NULL,
 next_value bigint NOT NULL CHECK(next_value BETWEEN 1 AND 1000000), PRIMARY KEY(company_id,year)
);
CREATE TABLE bank_movement (
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY, company_id bigint NOT NULL,
 account_id bigint NOT NULL, number text NOT NULL, revision bigint NOT NULL DEFAULT 1,
 source text NOT NULL CHECK(source IN ('csv','manual')), external_id text NOT NULL,
 data jsonb NOT NULL, amount numeric(14,2) NOT NULL CHECK(amount<>0), booked_on date NOT NULL,
 created_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(company_id,id), UNIQUE(company_id,number), UNIQUE(company_id,account_id,external_id),
 FOREIGN KEY(company_id,account_id) REFERENCES bank_account(company_id,id)
);
CREATE TABLE bank_match (
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY, company_id bigint NOT NULL, movement_id bigint NOT NULL,
 invoice_id bigint, received_id bigint, payment_id text NOT NULL,
 created_payment boolean NOT NULL, actor_id bigint NOT NULL REFERENCES app_user(id),
 created_at timestamptz NOT NULL DEFAULT now(), detached_at timestamptz,
 detached_by bigint REFERENCES app_user(id), reason text,
 CHECK(num_nonnulls(invoice_id,received_id)=1),
 FOREIGN KEY(company_id,movement_id) REFERENCES bank_movement(company_id,id),
 FOREIGN KEY(company_id,invoice_id) REFERENCES invoice_draft(company_id,id),
 FOREIGN KEY(company_id,received_id) REFERENCES received_invoice(company_id,id)
);
CREATE UNIQUE INDEX bank_match_movement ON bank_match(company_id,movement_id) WHERE detached_at IS NULL;
CREATE UNIQUE INDEX bank_match_invoice_payment ON bank_match(company_id,invoice_id,payment_id) WHERE detached_at IS NULL AND invoice_id IS NOT NULL;
CREATE UNIQUE INDEX bank_match_received_payment ON bank_match(company_id,received_id,payment_id) WHERE detached_at IS NULL AND received_id IS NOT NULL;
CREATE TABLE bank_command (
 company_id bigint NOT NULL REFERENCES company(id), request_id uuid NOT NULL,
 actor_id bigint NOT NULL REFERENCES app_user(id), input jsonb NOT NULL, result jsonb NOT NULL,
 created_at timestamptz NOT NULL DEFAULT now(), PRIMARY KEY(company_id,request_id)
);
CREATE INDEX bank_movement_register ON bank_movement(company_id,booked_on DESC,id DESC);
CREATE INDEX bank_match_history ON bank_match(company_id,movement_id,id);
CREATE INDEX bank_movement_account_register ON bank_movement(company_id,account_id,booked_on DESC,id DESC);
