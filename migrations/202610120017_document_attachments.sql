-- Files live in PostgreSQL so document/outbox references and backups are atomic.
CREATE TABLE document_attachment (
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 company_id bigint NOT NULL REFERENCES company(id),
 invoice_id bigint, quote_id bigint,
 request_id uuid NOT NULL, actor_id bigint NOT NULL REFERENCES app_user(id),
 filename text NOT NULL, media_type text NOT NULL,
 size bigint NOT NULL CHECK(size BETWEEN 1 AND 2097152), sha256 text NOT NULL,
 content bytea, created_at timestamptz NOT NULL DEFAULT now(), deleted_at timestamptz,
 CHECK(num_nonnulls(invoice_id,quote_id)=1),
 CHECK(content IS NOT NULL OR deleted_at IS NOT NULL),
 UNIQUE(company_id,id), UNIQUE(company_id,request_id),
 FOREIGN KEY(company_id,invoice_id) REFERENCES invoice_draft(company_id,id),
 FOREIGN KEY(company_id,quote_id) REFERENCES sales_document(company_id,id)
);
CREATE INDEX document_attachment_invoice ON document_attachment(company_id,invoice_id) WHERE deleted_at IS NULL;
CREATE INDEX document_attachment_quote ON document_attachment(company_id,quote_id) WHERE deleted_at IS NULL;
ALTER TABLE invoice_delivery ADD COLUMN attachments jsonb NOT NULL DEFAULT '[]';
ALTER TABLE quote_delivery ADD COLUMN attachments jsonb NOT NULL DEFAULT '[]';
CREATE TABLE delivery_attachment (
 company_id bigint NOT NULL, attachment_id bigint NOT NULL,
 invoice_delivery_id bigint, quote_delivery_id bigint,
 CHECK(num_nonnulls(invoice_delivery_id,quote_delivery_id)=1),
 FOREIGN KEY(company_id,attachment_id) REFERENCES document_attachment(company_id,id),
 FOREIGN KEY(company_id,invoice_delivery_id) REFERENCES invoice_delivery(company_id,id),
 FOREIGN KEY(company_id,quote_delivery_id) REFERENCES quote_delivery(company_id,id),
 UNIQUE(invoice_delivery_id,attachment_id), UNIQUE(quote_delivery_id,attachment_id)
);
CREATE INDEX delivery_attachment_file ON delivery_attachment(company_id,attachment_id);
CREATE TABLE attachment_audit (
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 company_id bigint NOT NULL, attachment_id bigint NOT NULL,
 actor_id bigint NOT NULL REFERENCES app_user(id), action text NOT NULL,
 created_at timestamptz NOT NULL DEFAULT now(),
 FOREIGN KEY(company_id,attachment_id) REFERENCES document_attachment(company_id,id)
);
