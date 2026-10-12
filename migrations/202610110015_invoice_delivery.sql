-- Additive outbox. Issued invoice rows and their immutable PDF cache are unchanged.
CREATE TABLE invoice_delivery (
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 company_id bigint NOT NULL, document_id bigint NOT NULL,
 actor_id bigint NOT NULL REFERENCES app_user(id),
 request_id uuid NOT NULL, command jsonb NOT NULL,
 revision bigint NOT NULL, snapshot jsonb NOT NULL, pdf bytea NOT NULL,
 sender text NOT NULL, reply_to text NOT NULL,
 recipient text NOT NULL, subject text NOT NULL, body text NOT NULL,
 message_id text NOT NULL UNIQUE,
 status text NOT NULL DEFAULT 'queued' CHECK(status IN ('queued','sending','sent','failed','uncertain')),
 error_code text, created_at timestamptz NOT NULL DEFAULT now(),
 started_at timestamptz, finished_at timestamptz,
 UNIQUE(company_id,id), UNIQUE(company_id,request_id),
 FOREIGN KEY(company_id,document_id) REFERENCES invoice_draft(company_id,id)
);
CREATE INDEX invoice_delivery_queue ON invoice_delivery(created_at,id) WHERE status='queued';
CREATE INDEX invoice_delivery_history ON invoice_delivery(company_id,document_id,id DESC);
