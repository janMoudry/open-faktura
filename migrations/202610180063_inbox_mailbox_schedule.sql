ALTER TABLE inbox_mailbox_settings ADD COLUMN automatic boolean NOT NULL DEFAULT false;
ALTER TABLE inbox_mailbox_settings ADD COLUMN next_run_at timestamptz;
ALTER TABLE inbox_mailbox_settings ADD CONSTRAINT mailbox_automatic_enabled CHECK(NOT automatic OR enabled);
ALTER TABLE inbox_mailbox_settings_audit ADD COLUMN automatic boolean NOT NULL DEFAULT false;
CREATE INDEX inbox_mailbox_due ON inbox_mailbox_settings(next_run_at) WHERE enabled AND automatic;
