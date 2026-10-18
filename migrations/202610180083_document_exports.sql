-- Export locking freezes document content, never its payments, balances or PDF cache.
CREATE TABLE document_export (
 id uuid NOT NULL,company_id bigint NOT NULL REFERENCES company(id),
 request_id uuid NOT NULL,actor_id bigint NOT NULL REFERENCES app_user(id),
 input jsonb NOT NULL,content jsonb NOT NULL,sha256 text NOT NULL,
 created_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(company_id,id),UNIQUE(company_id,request_id)
);
CREATE TABLE document_export_lock (
 company_id bigint NOT NULL REFERENCES company(id),kind text NOT NULL CHECK(kind IN ('invoice','sales','received')),
 document_id bigint NOT NULL,export_id uuid NOT NULL,
 PRIMARY KEY(company_id,kind,document_id),
 FOREIGN KEY(company_id,export_id) REFERENCES document_export(company_id,id)
);
CREATE TABLE document_export_event (
 company_id bigint NOT NULL REFERENCES company(id),request_id uuid NOT NULL,
 actor_id bigint NOT NULL REFERENCES app_user(id),action text NOT NULL CHECK(action IN ('export','unlock')),
 input jsonb NOT NULL,result jsonb NOT NULL,reason text,
 created_at timestamptz NOT NULL DEFAULT now(),PRIMARY KEY(company_id,request_id)
);
CREATE TRIGGER document_export_immutable BEFORE UPDATE OR DELETE ON document_export FOR EACH ROW EXECUTE FUNCTION protect_payment_history();
CREATE TRIGGER document_export_event_immutable BEFORE UPDATE OR DELETE ON document_export_event FOR EACH ROW EXECUTE FUNCTION protect_payment_history();
CREATE FUNCTION protect_exported_document() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE source_kind text; changed boolean;
BEGIN
 source_kind=CASE TG_TABLE_NAME WHEN 'invoice_draft' THEN 'invoice' WHEN 'sales_document' THEN 'sales' ELSE 'received' END;
 IF TG_OP='DELETE' THEN changed=true;
 ELSIF source_kind='invoice' THEN changed=NEW.data IS DISTINCT FROM OLD.data OR NEW.totals IS DISTINCT FROM OLD.totals OR NEW.number IS DISTINCT FROM OLD.number OR NEW.advances IS DISTINCT FROM OLD.advances OR NEW.document_style IS DISTINCT FROM OLD.document_style;
 ELSIF source_kind='sales' THEN changed=NEW.data IS DISTINCT FROM OLD.data OR NEW.kind IS DISTINCT FROM OLD.kind OR NEW.number IS DISTINCT FROM OLD.number;
 ELSE changed=NEW.data IS DISTINCT FROM OLD.data OR NEW.total IS DISTINCT FROM OLD.total OR NEW.number IS DISTINCT FROM OLD.number OR NEW.cancelled IS DISTINCT FROM OLD.cancelled OR NEW.supplier_contact_id IS DISTINCT FROM OLD.supplier_contact_id OR NEW.received_on IS DISTINCT FROM OLD.received_on OR NEW.due_on IS DISTINCT FROM OLD.due_on;
 END IF;
 IF changed AND EXISTS(SELECT 1 FROM document_export_lock l WHERE l.company_id=OLD.company_id AND l.kind=source_kind AND l.document_id=OLD.id) THEN
  RAISE EXCEPTION USING ERRCODE='23514',CONSTRAINT='document_export_locked',MESSAGE='document_export_locked';
 END IF;
 IF TG_OP='DELETE' THEN RETURN OLD; END IF;RETURN NEW;
END $$;
CREATE TRIGGER invoice_export_lock BEFORE UPDATE OR DELETE ON invoice_draft FOR EACH ROW EXECUTE FUNCTION protect_exported_document();
CREATE TRIGGER sales_export_lock BEFORE UPDATE OR DELETE ON sales_document FOR EACH ROW EXECUTE FUNCTION protect_exported_document();
CREATE TRIGGER received_export_lock BEFORE UPDATE OR DELETE ON received_invoice FOR EACH ROW EXECUTE FUNCTION protect_exported_document();
