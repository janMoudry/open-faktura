-- Reversible removal of unlinked, unpublished quotes/templates only. No hard delete.
CREATE TABLE sales_trash (
 company_id bigint NOT NULL, document_id bigint NOT NULL,
 deleted_at timestamptz,
 PRIMARY KEY(company_id,document_id),
 FOREIGN KEY(company_id,document_id) REFERENCES sales_document(company_id,id)
);
CREATE INDEX sales_trash_active ON sales_trash(company_id,deleted_at DESC) WHERE deleted_at IS NOT NULL;
CREATE TABLE sales_trash_command (
 company_id bigint NOT NULL, request_id uuid NOT NULL,
 actor_id bigint NOT NULL REFERENCES app_user(id), document_id bigint NOT NULL,
 command jsonb NOT NULL, result jsonb NOT NULL, created_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(company_id,request_id),
 FOREIGN KEY(company_id,document_id) REFERENCES sales_document(company_id,id)
);
CREATE FUNCTION protect_sales_trash_source() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE source_kind text; linked_invoice bigint;
BEGIN
 IF NEW.deleted_at IS NOT NULL THEN
  SELECT kind,invoice_id INTO source_kind,linked_invoice FROM sales_document
   WHERE company_id=NEW.company_id AND id=NEW.document_id FOR UPDATE;
  IF source_kind NOT IN ('quote','template') OR linked_invoice IS NOT NULL
   OR EXISTS(SELECT 1 FROM sales_conversion WHERE company_id=NEW.company_id AND document_id=NEW.document_id)
   OR EXISTS(SELECT 1 FROM quote_delivery WHERE company_id=NEW.company_id AND document_id=NEW.document_id)
   OR EXISTS(SELECT 1 FROM quote_share WHERE company_id=NEW.company_id AND document_id=NEW.document_id)
   OR EXISTS(SELECT 1 FROM document_export_lock WHERE company_id=NEW.company_id AND kind='sales' AND document_id=NEW.document_id) THEN
   RAISE EXCEPTION 'Sales document has protected history' USING ERRCODE='23514',CONSTRAINT='sales_trash_source_guard';
  END IF;
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER sales_trash_source_guard BEFORE INSERT OR UPDATE ON sales_trash
 FOR EACH ROW EXECUTE FUNCTION protect_sales_trash_source();
CREATE FUNCTION protect_trashed_sales() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF EXISTS(SELECT 1 FROM sales_trash WHERE company_id=NEW.company_id AND document_id=NEW.id AND deleted_at IS NOT NULL)
  AND (NEW.data IS DISTINCT FROM OLD.data OR NEW.kind IS DISTINCT FROM OLD.kind
   OR NEW.invoice_id IS DISTINCT FROM OLD.invoice_id OR NEW.archived IS DISTINCT FROM OLD.archived
   OR NEW.document_style IS DISTINCT FROM OLD.document_style) THEN
  RAISE EXCEPTION 'Sales document is in trash' USING ERRCODE='23514',CONSTRAINT='sales_trash_content_guard';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER sales_trash_content_guard BEFORE UPDATE ON sales_document
 FOR EACH ROW EXECUTE FUNCTION protect_trashed_sales();
CREATE TRIGGER sales_trash_command_immutable BEFORE UPDATE OR DELETE ON sales_trash_command
 FOR EACH ROW EXECUTE FUNCTION protect_payment_history();
