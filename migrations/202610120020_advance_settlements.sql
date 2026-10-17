ALTER TABLE invoice_draft ADD COLUMN advances jsonb;
CREATE TABLE advance_settlement_link (
 company_id bigint NOT NULL, advance_id bigint NOT NULL, invoice_id bigint NOT NULL,
 PRIMARY KEY(company_id,advance_id),
 FOREIGN KEY(company_id,advance_id) REFERENCES sales_document(company_id,id),
 FOREIGN KEY(company_id,invoice_id) REFERENCES invoice_draft(company_id,id)
);
CREATE INDEX advance_settlement_invoice ON advance_settlement_link(company_id,invoice_id);
CREATE TABLE advance_settlement_command (
 company_id bigint NOT NULL, request_id uuid NOT NULL, actor_id bigint NOT NULL REFERENCES app_user(id),
 command jsonb NOT NULL, invoice_id bigint NOT NULL, created_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(company_id,request_id), FOREIGN KEY(company_id,invoice_id) REFERENCES invoice_draft(company_id,id)
);
ALTER TABLE invoice_draft_audit ADD COLUMN advances jsonb;
-- Append the new column after the existing view columns to preserve dependent readers.
CREATE OR REPLACE VIEW invoice_with_settlement AS
SELECT d.id,d.company_id,d.contact_id,d.creator_id,d.request_id,d.initial_data,d.data,d.totals,d.revision,d.created_at,d.updated_at,d.number,d.issued_at,d.issue_request_id,d.issue_actor_id,d.issue_revision,d.pdf,
 CASE WHEN s.invoice_id IS NULL THEN NULL ELSE jsonb_build_object(
 'paid',s.paid::text,'outstanding',s.outstanding::text,'overpayment',s.overpayment::text,
 'status',s.payment_status,'overdue',s.overdue,'revision',s.payment_revision,'today',s.today::text) END AS settlement,
 d.advances
FROM invoice_draft d LEFT JOIN invoice_settlement s ON s.company_id=d.company_id AND s.invoice_id=d.id;
