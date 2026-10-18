-- Explicit original-file sends. No public bearer links or automatic retry.
CREATE TABLE inbox_delivery (
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 company_id bigint NOT NULL REFERENCES company(id),document_id bigint NOT NULL,
 actor_id bigint NOT NULL REFERENCES app_user(id),request_id uuid NOT NULL,
 command jsonb NOT NULL,revision bigint NOT NULL,
 sender text NOT NULL,recipient text NOT NULL,subject text NOT NULL,body text NOT NULL,message_id text NOT NULL,
 filename text NOT NULL,media_type text NOT NULL,sha256 text NOT NULL,size bigint NOT NULL CHECK(size BETWEEN 1 AND 2097152),content bytea NOT NULL,
 status text NOT NULL DEFAULT 'queued' CHECK(status IN ('queued','sending','sent','failed','uncertain')),
 error_code text,created_at timestamptz NOT NULL DEFAULT now(),started_at timestamptz,finished_at timestamptz,
 CHECK(octet_length(content)=size),
 UNIQUE(company_id,request_id),FOREIGN KEY(company_id,document_id) REFERENCES inbox_document(company_id,id)
);
CREATE INDEX inbox_delivery_queue ON inbox_delivery(id) WHERE status='queued';
CREATE INDEX inbox_delivery_history ON inbox_delivery(company_id,document_id,id);
