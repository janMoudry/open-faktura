-- Explicit opt-ins and authenticated gateway intake, no provider secrets in DB.
CREATE TABLE inbox_settings (
 company_id bigint PRIMARY KEY REFERENCES company(id),
 revision bigint NOT NULL DEFAULT 1,
 email_enabled boolean NOT NULL DEFAULT false,
 extraction_provider text,
 actor_id bigint NOT NULL REFERENCES app_user(id),
 updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE inbox_mail (
 company_id bigint NOT NULL REFERENCES company(id),
 message_id text NOT NULL,
 request_id uuid NOT NULL,
 actor_id bigint NOT NULL REFERENCES app_user(id),
 input_hash text NOT NULL,
 sender text NOT NULL, subject text NOT NULL,
 status text NOT NULL CHECK(status IN ('processing','completed','failed')),
 attachment_count integer NOT NULL,
 result jsonb,
 created_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(company_id,message_id), UNIQUE(company_id,request_id)
);
CREATE TABLE inbox_settings_audit (
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 company_id bigint NOT NULL REFERENCES company(id),
 actor_id bigint NOT NULL REFERENCES app_user(id), revision bigint NOT NULL,
 email_enabled boolean NOT NULL, extraction_provider text,
 created_at timestamptz NOT NULL DEFAULT now(), UNIQUE(company_id,revision)
);
