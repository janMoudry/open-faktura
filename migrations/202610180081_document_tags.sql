-- Tags are metadata, never a rewrite of issued document snapshots.
CREATE TABLE document_tag (
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 company_id bigint NOT NULL REFERENCES company(id),
 name text NOT NULL CHECK(length(name) BETWEEN 1 AND 80 AND name=btrim(name)),
 revision bigint NOT NULL DEFAULT 1 CHECK(revision>0),
 UNIQUE(company_id,id)
);
CREATE UNIQUE INDEX document_tag_name ON document_tag(company_id,lower(name));
CREATE TABLE document_tag_link (
 company_id bigint NOT NULL,
 tag_id bigint NOT NULL,
 invoice_id bigint, sales_id bigint, received_id bigint, contact_id bigint,
 FOREIGN KEY(company_id,tag_id) REFERENCES document_tag(company_id,id) ON DELETE CASCADE,
 FOREIGN KEY(company_id,invoice_id) REFERENCES invoice_draft(company_id,id),
 FOREIGN KEY(company_id,sales_id) REFERENCES sales_document(company_id,id),
 FOREIGN KEY(company_id,received_id) REFERENCES received_invoice(company_id,id),
 FOREIGN KEY(company_id,contact_id) REFERENCES contact(company_id,id),
 CHECK(num_nonnulls(invoice_id,sales_id,received_id,contact_id)=1)
);
CREATE UNIQUE INDEX tag_invoice ON document_tag_link(company_id,tag_id,invoice_id) WHERE invoice_id IS NOT NULL;
CREATE UNIQUE INDEX tag_sales ON document_tag_link(company_id,tag_id,sales_id) WHERE sales_id IS NOT NULL;
CREATE UNIQUE INDEX tag_received ON document_tag_link(company_id,tag_id,received_id) WHERE received_id IS NOT NULL;
CREATE UNIQUE INDEX tag_contact ON document_tag_link(company_id,tag_id,contact_id) WHERE contact_id IS NOT NULL;
CREATE TABLE document_tag_command (
 company_id bigint NOT NULL REFERENCES company(id), request_id uuid NOT NULL,
 actor_id bigint NOT NULL REFERENCES app_user(id),input jsonb NOT NULL,result jsonb NOT NULL,
 created_at timestamptz NOT NULL DEFAULT now(),PRIMARY KEY(company_id,request_id)
);
