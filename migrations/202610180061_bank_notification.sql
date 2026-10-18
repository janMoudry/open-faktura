-- Custom authenticated gateway intake: opt-in independent of auto matching.
CREATE TABLE bank_notification_settings (
 company_id bigint NOT NULL, account_id bigint NOT NULL,
 revision bigint NOT NULL DEFAULT 1, enabled boolean NOT NULL DEFAULT false,
 format text NOT NULL CHECK(format IN ('camt053','fio_json','moneta_xml','rb_abo')),
 actor_id bigint NOT NULL REFERENCES app_user(id), updated_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(company_id,account_id),
 FOREIGN KEY(company_id,account_id) REFERENCES bank_account(company_id,id)
);
CREATE TABLE bank_notification (
 company_id bigint NOT NULL, account_id bigint NOT NULL,
 message_id text NOT NULL, request_id uuid NOT NULL,
 actor_id bigint NOT NULL REFERENCES app_user(id), sender text NOT NULL,
 input_hash text NOT NULL, result jsonb NOT NULL,
 new_count integer NOT NULL, duplicate_count integer NOT NULL,
 created_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(company_id,account_id,message_id), UNIQUE(company_id,request_id),
 FOREIGN KEY(company_id,account_id) REFERENCES bank_account(company_id,id)
);
CREATE TABLE bank_notification_settings_audit (
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 company_id bigint NOT NULL, account_id bigint NOT NULL,
 actor_id bigint NOT NULL REFERENCES app_user(id), revision bigint NOT NULL,
 enabled boolean NOT NULL, format text NOT NULL, created_at timestamptz NOT NULL DEFAULT now(),
 FOREIGN KEY(company_id,account_id) REFERENCES bank_account(company_id,id),
 UNIQUE(company_id,account_id,revision)
);
