-- Additive P06 draft storage. No invoice numbers or issued documents yet.
CREATE TABLE invoice_draft (
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 company_id bigint NOT NULL REFERENCES company(id),
 contact_id bigint,
 creator_id bigint NOT NULL REFERENCES app_user(id),
 request_id uuid NOT NULL,
 initial_data jsonb NOT NULL,
 data jsonb NOT NULL,
 totals jsonb,
 revision bigint NOT NULL DEFAULT 1 CHECK(revision>0),
 created_at timestamptz NOT NULL DEFAULT now(),
 updated_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(company_id,request_id), UNIQUE(company_id,id),
 FOREIGN KEY(company_id,contact_id) REFERENCES contact(company_id,id)
);
CREATE INDEX invoice_draft_list ON invoice_draft(company_id,updated_at DESC,id DESC);
CREATE INDEX invoice_draft_contact ON invoice_draft(company_id,contact_id);
CREATE TABLE invoice_draft_audit (
 company_id bigint NOT NULL, draft_id bigint NOT NULL, revision bigint NOT NULL,
 actor_id bigint NOT NULL REFERENCES app_user(id),
 action text NOT NULL CHECK(action IN ('create','update')),
 data jsonb NOT NULL, totals jsonb, created_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(company_id,draft_id,revision),
 FOREIGN KEY(company_id,draft_id) REFERENCES invoice_draft(company_id,id)
);
