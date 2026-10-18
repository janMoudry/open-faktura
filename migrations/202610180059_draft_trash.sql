-- Reversible trash for unissued, unlinked invoice drafts; no historical deletion.
CREATE TABLE invoice_trash (
 company_id bigint NOT NULL, draft_id bigint NOT NULL, deleted_at timestamptz,
 PRIMARY KEY(company_id,draft_id),
 FOREIGN KEY(company_id,draft_id) REFERENCES invoice_draft(company_id,id)
);
CREATE TABLE invoice_trash_command (
 company_id bigint NOT NULL, request_id uuid NOT NULL, actor_id bigint NOT NULL REFERENCES app_user(id),
 draft_id bigint NOT NULL, command jsonb NOT NULL, result jsonb NOT NULL,
 created_at timestamptz NOT NULL DEFAULT now(), PRIMARY KEY(company_id,request_id),
 FOREIGN KEY(company_id,draft_id) REFERENCES invoice_draft(company_id,id)
);
CREATE FUNCTION protect_trashed_draft() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.number IS NOT NULL AND EXISTS(SELECT 1 FROM invoice_trash t WHERE t.company_id=NEW.company_id AND t.draft_id=NEW.id AND t.deleted_at IS NOT NULL) THEN
  RAISE EXCEPTION 'Draft is in trash' USING ERRCODE='23514',CONSTRAINT='invoice_trash_issue_guard';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER invoice_trash_issue_guard BEFORE UPDATE ON invoice_draft FOR EACH ROW EXECUTE FUNCTION protect_trashed_draft();
CREATE FUNCTION protect_trash_source() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE issued_number text;
BEGIN
 IF NEW.deleted_at IS NOT NULL THEN
  SELECT number INTO issued_number FROM invoice_draft WHERE company_id=NEW.company_id AND id=NEW.draft_id FOR UPDATE;
  IF issued_number IS NOT NULL THEN
   RAISE EXCEPTION 'Issued document cannot be trashed' USING ERRCODE='23514',CONSTRAINT='invoice_trash_source_guard';
  END IF;
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER invoice_trash_source_guard BEFORE INSERT OR UPDATE ON invoice_trash FOR EACH ROW EXECUTE FUNCTION protect_trash_source();
CREATE TRIGGER invoice_trash_command_immutable BEFORE UPDATE OR DELETE ON invoice_trash_command FOR EACH ROW EXECUTE FUNCTION protect_payment_history();
