CREATE TABLE received_series (
 company_id bigint NOT NULL REFERENCES company(id), year integer NOT NULL,
 next_value bigint NOT NULL CHECK(next_value BETWEEN 1 AND 1000000), PRIMARY KEY(company_id,year)
);
CREATE TABLE received_invoice (
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY, company_id bigint NOT NULL REFERENCES company(id),
 supplier_contact_id bigint, number text NOT NULL, revision bigint NOT NULL DEFAULT 1,
 cancelled boolean NOT NULL DEFAULT false, data jsonb NOT NULL, payments jsonb NOT NULL DEFAULT '[]',
 total numeric(14,2) NOT NULL CHECK(total>0), paid numeric(14,2) NOT NULL DEFAULT 0 CHECK(paid>=0 AND paid<=total),
 received_on date NOT NULL, due_on date NOT NULL, created_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(company_id,id), UNIQUE(company_id,number),
 FOREIGN KEY(company_id,supplier_contact_id) REFERENCES contact(company_id,id)
);
CREATE TABLE received_command (
 company_id bigint NOT NULL REFERENCES company(id), request_id uuid NOT NULL,
 actor_id bigint NOT NULL REFERENCES app_user(id), invoice_id bigint NOT NULL,
 input jsonb NOT NULL, result jsonb NOT NULL,
 PRIMARY KEY(company_id,request_id), FOREIGN KEY(company_id,invoice_id) REFERENCES received_invoice(company_id,id)
);
CREATE TABLE received_audit (
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY, company_id bigint NOT NULL,
 invoice_id bigint NOT NULL, actor_id bigint NOT NULL REFERENCES app_user(id),
 revision bigint NOT NULL, action text NOT NULL, snapshot jsonb NOT NULL,
 created_at timestamptz NOT NULL DEFAULT now(),
 FOREIGN KEY(company_id,invoice_id) REFERENCES received_invoice(company_id,id)
);
CREATE INDEX received_register ON received_invoice(company_id,received_on DESC,id DESC);
ALTER TABLE document_attachment ADD COLUMN received_id bigint;
ALTER TABLE document_attachment DROP CONSTRAINT document_attachment_check;
ALTER TABLE document_attachment ADD CONSTRAINT attachment_one_document CHECK(num_nonnulls(invoice_id,quote_id,received_id)=1);
ALTER TABLE document_attachment ADD FOREIGN KEY(company_id,received_id) REFERENCES received_invoice(company_id,id);
CREATE INDEX document_attachment_received ON document_attachment(company_id,received_id) WHERE deleted_at IS NULL;
