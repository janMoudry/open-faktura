ALTER TABLE bank_connection
 ADD COLUMN schedule_enabled boolean NOT NULL DEFAULT false,
 ADD COLUMN interval_minutes integer NOT NULL DEFAULT 60 CHECK(interval_minutes IN (15,60,360,1440)),
 ADD COLUMN schedule_actor_id bigint REFERENCES app_user(id),
 ADD COLUMN next_run_at timestamptz,
 ADD COLUMN schedule_error text,
 ADD CONSTRAINT bank_schedule_enabled CHECK(NOT schedule_enabled OR (provider IS NOT NULL AND schedule_actor_id IS NOT NULL AND next_run_at IS NOT NULL));
ALTER TABLE bank_connection_run DROP CONSTRAINT bank_connection_run_kind_check;
ALTER TABLE bank_connection_run ADD CONSTRAINT bank_connection_run_kind_check CHECK(kind IN ('test','sync','save','disconnect','schedule'));
CREATE INDEX bank_schedule_due ON bank_connection(next_run_at) WHERE schedule_enabled;
