CREATE TABLE taxpayer_filing_profile (
 company_id bigint PRIMARY KEY REFERENCES company(id),
 revision bigint NOT NULL DEFAULT 1 CHECK(revision>0),
 data jsonb NOT NULL,
 updated_by bigint NOT NULL REFERENCES app_user(id),
 updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE taxpayer_filing_profile_command (
 company_id bigint NOT NULL REFERENCES company(id), request_id uuid NOT NULL,
 actor_id bigint NOT NULL REFERENCES app_user(id), input jsonb NOT NULL,
 result jsonb NOT NULL, created_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(company_id,request_id)
);
CREATE TABLE taxpayer_filing_profile_audit (
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 company_id bigint NOT NULL REFERENCES company(id), actor_id bigint NOT NULL REFERENCES app_user(id),
 revision bigint NOT NULL, data jsonb NOT NULL, created_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(company_id,revision)
);
