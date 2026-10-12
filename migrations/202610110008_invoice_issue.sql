-- Additive P07 migration: existing rows remain editable drafts.
ALTER TABLE invoice_draft ADD COLUMN number text,
 ADD COLUMN issued_at timestamptz,
 ADD COLUMN issue_request_id uuid,
 ADD COLUMN issue_actor_id bigint REFERENCES app_user(id),
 ADD COLUMN issue_revision bigint,
 ADD COLUMN pdf bytea,
 ADD CONSTRAINT invoice_issue_complete CHECK (
  (number IS NULL AND issued_at IS NULL AND issue_request_id IS NULL AND issue_actor_id IS NULL AND issue_revision IS NULL AND pdf IS NULL)
  OR (number IS NOT NULL AND issued_at IS NOT NULL AND issue_request_id IS NOT NULL AND issue_actor_id IS NOT NULL AND issue_revision IS NOT NULL AND totals IS NOT NULL AND totals <> 'null'::jsonb)),
 ADD UNIQUE(company_id,number), ADD UNIQUE(company_id,issue_request_id);
CREATE TABLE invoice_number_series (
 company_id bigint NOT NULL REFERENCES company(id), prefix text NOT NULL, year integer NOT NULL,
 next_value integer NOT NULL CHECK(next_value BETWEEN 1 AND 1000000),
 PRIMARY KEY(company_id,prefix,year)
);
ALTER TABLE invoice_draft_audit DROP CONSTRAINT invoice_draft_audit_action_check;
ALTER TABLE invoice_draft_audit ADD CHECK(action IN ('create','update','issue'));
ALTER TABLE invoice_draft_audit ADD COLUMN number text;
-- Protect against old application versions writing an already issued row.
CREATE FUNCTION protect_issued_invoice() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF OLD.number IS NOT NULL THEN
  IF TG_OP='DELETE' THEN RAISE EXCEPTION 'issued invoice is immutable'; END IF;
  IF (to_jsonb(NEW)-'pdf') IS DISTINCT FROM (to_jsonb(OLD)-'pdf') OR
     (OLD.pdf IS NOT NULL AND NEW.pdf IS DISTINCT FROM OLD.pdf) THEN
    RAISE EXCEPTION 'issued invoice is immutable';
  END IF;
 END IF;
 IF TG_OP='DELETE' THEN RETURN OLD; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER invoice_immutable BEFORE UPDATE OR DELETE ON invoice_draft
 FOR EACH ROW EXECUTE FUNCTION protect_issued_invoice();
