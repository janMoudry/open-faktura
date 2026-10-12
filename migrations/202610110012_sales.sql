CREATE TABLE sales_document (
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 company_id bigint NOT NULL REFERENCES company(id),
 kind text NOT NULL CHECK(kind IN ('template','quote')),
 creator_id bigint NOT NULL REFERENCES app_user(id),
 request_id uuid NOT NULL, initial_data jsonb NOT NULL, data jsonb NOT NULL,
 revision bigint NOT NULL DEFAULT 1 CHECK(revision>0), archived boolean NOT NULL DEFAULT false,
 number text, invoice_id bigint,
 created_at timestamptz NOT NULL DEFAULT now(), updated_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(company_id,id), UNIQUE(company_id,request_id), UNIQUE(company_id,number),
 FOREIGN KEY(company_id,invoice_id) REFERENCES invoice_draft(company_id,id)
);
CREATE TABLE quote_sequence (company_id bigint NOT NULL REFERENCES company(id),year integer NOT NULL,next_value bigint NOT NULL CHECK(next_value>0),PRIMARY KEY(company_id,year));
CREATE TABLE sales_audit (
 company_id bigint NOT NULL, document_id bigint NOT NULL, revision bigint NOT NULL,
 actor_id bigint NOT NULL REFERENCES app_user(id), action text NOT NULL,
 snapshot jsonb NOT NULL, created_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(company_id,document_id,revision),
 FOREIGN KEY(company_id,document_id) REFERENCES sales_document(company_id,id)
);
CREATE TABLE sales_conversion (
 company_id bigint NOT NULL, request_id uuid NOT NULL, document_id bigint NOT NULL,
 actor_id bigint NOT NULL REFERENCES app_user(id), command jsonb NOT NULL, invoice_id bigint NOT NULL,
 PRIMARY KEY(company_id,request_id),
 FOREIGN KEY(company_id,document_id) REFERENCES sales_document(company_id,id),
 FOREIGN KEY(company_id,invoice_id) REFERENCES invoice_draft(company_id,id)
);
CREATE INDEX sales_document_list ON sales_document(company_id,kind,archived,updated_at DESC,id DESC);
