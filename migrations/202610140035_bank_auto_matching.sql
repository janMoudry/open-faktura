ALTER TABLE bank_connection
 ADD COLUMN matching_enabled boolean NOT NULL DEFAULT false,
 ADD COLUMN matching_actor_id bigint REFERENCES app_user(id),
 ADD COLUMN matching_next_at timestamptz,
 ADD COLUMN matching_cursor bigint NOT NULL DEFAULT 0,
 ADD COLUMN matching_error text,
 ADD CONSTRAINT bank_matching_enabled CHECK(NOT matching_enabled OR (matching_actor_id IS NOT NULL AND matching_next_at IS NOT NULL));
ALTER TABLE bank_connection_run DROP CONSTRAINT bank_connection_run_kind_check;
ALTER TABLE bank_connection_run ADD CONSTRAINT bank_connection_run_kind_check CHECK(kind IN ('test','sync','save','disconnect','schedule','matching'));
CREATE INDEX bank_matching_due ON bank_connection(matching_next_at) WHERE matching_enabled;
CREATE TABLE bank_matching_run (
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 company_id bigint NOT NULL, account_id bigint NOT NULL,
 actor_id bigint NOT NULL REFERENCES app_user(id), configuration_revision bigint NOT NULL,
 created_at timestamptz NOT NULL DEFAULT now(), checked integer NOT NULL, matched integer NOT NULL,
 items jsonb NOT NULL,
 FOREIGN KEY(company_id,account_id) REFERENCES bank_connection(company_id,account_id)
);
CREATE INDEX bank_matching_history ON bank_matching_run(company_id,account_id,id DESC);
