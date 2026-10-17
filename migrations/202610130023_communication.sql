-- Additive: existing delivery commands and historical PDFs are untouched.
ALTER TABLE invoice_delivery ADD COLUMN purpose text NOT NULL DEFAULT 'document'
 CHECK(purpose IN ('document','reminder','thanks'));
ALTER TABLE invoice_delivery ADD COLUMN settlement_revision bigint;
ALTER TABLE invoice_delivery ADD COLUMN automation_revision bigint;
CREATE TABLE communication_settings (
 company_id bigint PRIMARY KEY REFERENCES company(id),
 revision bigint NOT NULL DEFAULT 1,
 actor_id bigint NOT NULL REFERENCES app_user(id),
 reminders boolean NOT NULL DEFAULT false,
 thanks boolean NOT NULL DEFAULT false,
 first_days integer NOT NULL CHECK(first_days BETWEEN 1 AND 365),
 repeat_days integer NOT NULL CHECK(repeat_days BETWEEN 1 AND 365),
 minimum_total numeric(30,2) NOT NULL CHECK(minimum_total>=0),
 thanks_since timestamptz NOT NULL DEFAULT now(),
 updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE communication_pause (
 company_id bigint NOT NULL, invoice_id bigint NOT NULL,
 paused boolean NOT NULL, actor_id bigint NOT NULL REFERENCES app_user(id),
 updated_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(company_id,invoice_id),
 FOREIGN KEY(company_id,invoice_id) REFERENCES invoice_draft(company_id,id)
);
CREATE TABLE communication_job (
 company_id bigint NOT NULL, invoice_id bigint NOT NULL,
 purpose text NOT NULL CHECK(purpose IN ('reminder','thanks')),
 slot integer NOT NULL, request_id uuid NOT NULL UNIQUE,
 settings_revision bigint NOT NULL,
 command jsonb NOT NULL, actor_id bigint NOT NULL REFERENCES app_user(id),
 delivery_id bigint, error_code text, next_attempt timestamptz NOT NULL DEFAULT now(), created_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(company_id,invoice_id,purpose,slot),
 FOREIGN KEY(company_id,invoice_id) REFERENCES invoice_draft(company_id,id),
 FOREIGN KEY(company_id,delivery_id) REFERENCES invoice_delivery(company_id,id)
);
CREATE INDEX communication_jobs_pending ON communication_job(created_at) WHERE delivery_id IS NULL AND error_code IS NULL;
CREATE TABLE communication_audit (
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 company_id bigint NOT NULL REFERENCES company(id),
 actor_id bigint NOT NULL REFERENCES app_user(id),
 invoice_id bigint, input jsonb NOT NULL, created_at timestamptz NOT NULL DEFAULT now(),
 FOREIGN KEY(company_id,invoice_id) REFERENCES invoice_draft(company_id,id)
);
CREATE INDEX communication_delivery_history ON invoice_delivery(company_id,document_id,purpose,created_at);
