-- Additive P09. Never rewrite source documents or reserve new invoice numbers.
CREATE TABLE import_batch (
 id uuid PRIMARY KEY, company_id bigint NOT NULL REFERENCES company(id),
 actor_id bigint NOT NULL REFERENCES app_user(id), source text NOT NULL CHECK(source IN ('idoklad','fakturaonline','generic')),
 agenda text NOT NULL, input jsonb NOT NULL, status text NOT NULL DEFAULT 'preview' CHECK(status IN ('preview','running','done')),
 created_at timestamptz NOT NULL DEFAULT now(), updated_at timestamptz NOT NULL DEFAULT now(), UNIQUE(company_id,id)
);
CREATE INDEX import_batch_profile ON import_batch(company_id,created_at DESC);
CREATE TABLE import_row (
 company_id bigint NOT NULL, batch_id uuid NOT NULL, position integer NOT NULL,
 kind text NOT NULL, source_key text NOT NULL, label text NOT NULL, data jsonb NOT NULL,
 status text NOT NULL CHECK(status IN ('ready','invalid','conflict','duplicate','imported','skipped')),
 fields jsonb NOT NULL, target_id bigint, PRIMARY KEY(batch_id,position),
 FOREIGN KEY(company_id,batch_id) REFERENCES import_batch(company_id,id)
);
CREATE TABLE import_identity (
 company_id bigint NOT NULL REFERENCES company(id), source text NOT NULL, agenda text NOT NULL,
 kind text NOT NULL CHECK(kind IN ('contact','invoice')), source_key text NOT NULL, data jsonb NOT NULL,
 contact_id bigint, invoice_id bigint, batch_id uuid NOT NULL, position integer NOT NULL,
 PRIMARY KEY(company_id,source,agenda,kind,source_key),
 CHECK((kind='contact' AND contact_id IS NOT NULL AND invoice_id IS NULL) OR (kind='invoice' AND invoice_id IS NOT NULL AND contact_id IS NULL)),
 FOREIGN KEY(company_id,contact_id) REFERENCES contact(company_id,id),
 FOREIGN KEY(company_id,invoice_id) REFERENCES invoice_draft(company_id,id),
 FOREIGN KEY(company_id,batch_id) REFERENCES import_batch(company_id,id),
 FOREIGN KEY(batch_id,position) REFERENCES import_row(batch_id,position)
);
CREATE TRIGGER import_identity_immutable BEFORE UPDATE OR DELETE ON import_identity FOR EACH ROW EXECUTE FUNCTION protect_payment_history();
