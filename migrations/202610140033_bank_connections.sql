CREATE TABLE bank_connection (
 company_id bigint NOT NULL, account_id bigint NOT NULL,
 revision bigint NOT NULL DEFAULT 0, provider text, secret bytea,
 updated_by bigint REFERENCES app_user(id), updated_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(company_id,account_id),
 FOREIGN KEY(company_id,account_id) REFERENCES bank_account(company_id,id),
 CHECK ((provider IS NULL) = (secret IS NULL))
);
CREATE TABLE bank_connection_run (
 company_id bigint NOT NULL, account_id bigint NOT NULL, request_id uuid NOT NULL,
 actor_id bigint NOT NULL REFERENCES app_user(id), revision bigint NOT NULL,
 kind text NOT NULL CHECK(kind IN ('test','sync','save','disconnect')),
 status text NOT NULL CHECK(status IN ('running','success','failed')),
 started_at timestamptz NOT NULL DEFAULT now(), finished_at timestamptz,
 error_code text, imported integer NOT NULL DEFAULT 0, duplicates integer NOT NULL DEFAULT 0,
 PRIMARY KEY(company_id,account_id,request_id),
 FOREIGN KEY(company_id,account_id) REFERENCES bank_connection(company_id,account_id)
);
CREATE INDEX bank_connection_run_latest ON bank_connection_run(company_id,account_id,started_at DESC);
