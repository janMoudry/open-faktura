CREATE TABLE inbox_document (
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 company_id bigint NOT NULL REFERENCES company(id),
 actor_id bigint NOT NULL REFERENCES app_user(id),
 filename text NOT NULL, media_type text NOT NULL,
 size bigint NOT NULL CHECK(size BETWEEN 1 AND 2097152),
 sha256 text NOT NULL, content bytea NOT NULL,
 revision bigint NOT NULL DEFAULT 1,
 archived boolean NOT NULL DEFAULT false,
 received_id bigint, created_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(company_id,id), UNIQUE(company_id,sha256),
 FOREIGN KEY(company_id,received_id) REFERENCES received_invoice(company_id,id)
);
CREATE TABLE inbox_command (
 company_id bigint NOT NULL REFERENCES company(id), request_id uuid NOT NULL,
 actor_id bigint NOT NULL REFERENCES app_user(id), input jsonb NOT NULL,result jsonb NOT NULL,
 created_at timestamptz NOT NULL DEFAULT now(), PRIMARY KEY(company_id,request_id)
);
CREATE TABLE inbox_audit (
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 company_id bigint NOT NULL, document_id bigint NOT NULL, actor_id bigint NOT NULL REFERENCES app_user(id),
 revision bigint NOT NULL, action text NOT NULL, created_at timestamptz NOT NULL DEFAULT now(),
 FOREIGN KEY(company_id,document_id) REFERENCES inbox_document(company_id,id)
);
