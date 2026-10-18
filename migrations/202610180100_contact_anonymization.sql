-- Directory-only anonymization; financial document snapshots remain unchanged.
ALTER TABLE contact ADD COLUMN anonymized_at timestamptz;
ALTER TABLE contact_audit DROP CONSTRAINT contact_audit_action_check;
ALTER TABLE contact_audit ADD CONSTRAINT contact_audit_action_check
 CHECK(action IN ('create','update','archive','restore','anonymize'));
CREATE TABLE contact_anonymization_command (
 company_id bigint NOT NULL, request_id uuid NOT NULL,
 contact_id bigint NOT NULL, actor_id bigint NOT NULL REFERENCES app_user(id),
 command jsonb NOT NULL, result jsonb NOT NULL, created_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(company_id,request_id),
 FOREIGN KEY(company_id,contact_id) REFERENCES contact(company_id,id)
);
CREATE FUNCTION protect_anonymized_contact() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF OLD.anonymized_at IS NOT NULL AND
  (NEW.data IS DISTINCT FROM OLD.data OR NEW.initial_data IS DISTINCT FROM OLD.initial_data
   OR NOT NEW.archived OR NEW.anonymized_at IS DISTINCT FROM OLD.anonymized_at) THEN
  RAISE EXCEPTION 'anonymized contact is immutable';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER contact_anonymized_immutable BEFORE UPDATE ON contact
 FOR EACH ROW EXECUTE FUNCTION protect_anonymized_contact();
