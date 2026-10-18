-- New invoice snapshots only. Old cached PDFs and issued snapshots stay unchanged.
CREATE TABLE company_document_style (
 company_id bigint PRIMARY KEY REFERENCES company(id),revision bigint NOT NULL DEFAULT 1,
 data jsonb NOT NULL CHECK(jsonb_typeof(data)='object'),
 updated_by bigint NOT NULL REFERENCES app_user(id),updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE document_style_command (
 company_id bigint NOT NULL REFERENCES company(id),request_id uuid NOT NULL,
 actor_id bigint NOT NULL REFERENCES app_user(id),input jsonb NOT NULL,result jsonb NOT NULL,
 created_at timestamptz NOT NULL DEFAULT now(),PRIMARY KEY(company_id,request_id)
);
ALTER TABLE invoice_draft ADD COLUMN document_style jsonb;
