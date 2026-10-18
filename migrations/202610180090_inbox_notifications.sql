-- Own-user opt-in and per-category read cursors, never company write privileges.
CREATE TABLE inbox_notification_settings (
 company_id bigint NOT NULL REFERENCES company(id), user_id bigint NOT NULL REFERENCES app_user(id),
 revision bigint NOT NULL DEFAULT 1, new_documents boolean NOT NULL DEFAULT false, errors boolean NOT NULL DEFAULT false,
 documents_read bigint NOT NULL DEFAULT 0, errors_read bigint NOT NULL DEFAULT 0,
 PRIMARY KEY(company_id,user_id)
);
CREATE TABLE inbox_notification_event (
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY, company_id bigint NOT NULL REFERENCES company(id),
 category text NOT NULL CHECK(category IN ('document','error')),
 document_id bigint, filename text, code text, source_key uuid,
 created_at timestamptz NOT NULL DEFAULT now(),
 FOREIGN KEY(company_id,document_id) REFERENCES inbox_document(company_id,id),
 CHECK((category='document' AND document_id IS NOT NULL AND filename IS NOT NULL AND code IS NULL) OR
       (category='error' AND document_id IS NULL AND filename IS NULL AND code IN ('mailbox_failed','message_rejected','gateway_failed')))
);
CREATE UNIQUE INDEX inbox_notification_gateway_replay ON inbox_notification_event(company_id,source_key) WHERE source_key IS NOT NULL;
CREATE INDEX inbox_notification_company ON inbox_notification_event(company_id,id);
-- The company lock precedes identity allocation so a read cursor cannot skip an
-- older identity that commits later. Shared with inbox writes; no network I/O.
CREATE FUNCTION inbox_notify() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 PERFORM pg_advisory_xact_lock(NEW.company_id);
 IF TG_TABLE_NAME='inbox_document' THEN
  INSERT INTO inbox_notification_event(company_id,category,document_id,filename) VALUES(NEW.company_id,'document',NEW.id,NEW.filename);
 ELSIF TG_TABLE_NAME='inbox_mailbox_message' AND NEW.status='rejected' THEN
  INSERT INTO inbox_notification_event(company_id,category,code) VALUES(NEW.company_id,'error','message_rejected');
 ELSIF TG_TABLE_NAME='inbox_mailbox_run' AND NEW.status='failed' AND (TG_OP='INSERT' OR OLD.status IS DISTINCT FROM NEW.status) THEN
  INSERT INTO inbox_notification_event(company_id,category,code) VALUES(NEW.company_id,'error','mailbox_failed');
 ELSIF TG_TABLE_NAME='inbox_mail' AND NEW.status='failed' AND (TG_OP='INSERT' OR OLD.status IS DISTINCT FROM NEW.status) THEN
  INSERT INTO inbox_notification_event(company_id,category,code) VALUES(NEW.company_id,'error','gateway_failed');
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER inbox_notification_document AFTER INSERT ON inbox_document FOR EACH ROW EXECUTE FUNCTION inbox_notify();
CREATE TRIGGER inbox_notification_message AFTER INSERT ON inbox_mailbox_message FOR EACH ROW EXECUTE FUNCTION inbox_notify();
CREATE TRIGGER inbox_notification_run AFTER INSERT OR UPDATE ON inbox_mailbox_run FOR EACH ROW EXECUTE FUNCTION inbox_notify();
CREATE TRIGGER inbox_notification_gateway AFTER INSERT OR UPDATE ON inbox_mail FOR EACH ROW EXECUTE FUNCTION inbox_notify();
