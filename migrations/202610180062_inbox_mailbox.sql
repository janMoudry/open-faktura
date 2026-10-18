CREATE TABLE inbox_mailbox_settings (
 company_id bigint PRIMARY KEY REFERENCES company(id),revision bigint NOT NULL DEFAULT 1,
 enabled boolean NOT NULL DEFAULT false,provider text,
 uid_validity bigint CHECK(uid_validity BETWEEN 1 AND 4294967295),cursor bigint NOT NULL DEFAULT 0 CHECK(cursor BETWEEN 0 AND 4294967295),
 actor_id bigint NOT NULL REFERENCES app_user(id),
 running_request uuid,claimed_at timestamptz,
 updated_at timestamptz NOT NULL DEFAULT now(),CHECK(NOT enabled OR provider IS NOT NULL)
);
CREATE TABLE inbox_mailbox_settings_audit (
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 company_id bigint NOT NULL REFERENCES company(id),actor_id bigint NOT NULL REFERENCES app_user(id),
 revision bigint NOT NULL,enabled boolean NOT NULL,provider text,
 created_at timestamptz NOT NULL DEFAULT now(),UNIQUE(company_id,revision)
);
CREATE TABLE inbox_mailbox_run (
 company_id bigint NOT NULL REFERENCES company(id),request_id uuid NOT NULL,
 actor_id bigint NOT NULL REFERENCES app_user(id),settings_revision bigint NOT NULL,
 status text NOT NULL CHECK(status IN ('running','completed','failed')),error text,
 result jsonb,created_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(company_id,request_id)
);
CREATE TABLE inbox_mailbox_message (
 company_id bigint NOT NULL REFERENCES company(id),provider text NOT NULL,
 uid_validity bigint NOT NULL,uid bigint NOT NULL,
 status text NOT NULL CHECK(status IN ('imported','rejected')),error text,
 created_at timestamptz NOT NULL DEFAULT now(),PRIMARY KEY(company_id,provider,uid_validity,uid)
);
