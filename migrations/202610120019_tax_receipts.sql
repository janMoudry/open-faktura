CREATE TABLE tax_receipt_sequence (
 company_id bigint NOT NULL REFERENCES company(id), year integer NOT NULL,
 next_value bigint NOT NULL CHECK(next_value>0), PRIMARY KEY(company_id,year)
);
CREATE TABLE tax_receipt (
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 company_id bigint NOT NULL, advance_id bigint NOT NULL, payment_id bigint NOT NULL,
 actor_id bigint NOT NULL REFERENCES app_user(id), request_id uuid NOT NULL,
 command jsonb NOT NULL, number text NOT NULL, snapshot jsonb NOT NULL, pdf bytea NOT NULL,
 created_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(company_id,id), UNIQUE(company_id,number), UNIQUE(company_id,request_id),
 UNIQUE(company_id,payment_id),
 FOREIGN KEY(company_id,advance_id,payment_id) REFERENCES advance_payment(company_id,document_id,id)
);
CREATE INDEX tax_receipt_advance ON tax_receipt(company_id,advance_id,id);
