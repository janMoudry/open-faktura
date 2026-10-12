-- P08 append-only receipts and reversals; issued invoice/PDF never changes.
CREATE TABLE invoice_payment (
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 company_id bigint NOT NULL, invoice_id bigint NOT NULL,
 amount numeric(20,2) NOT NULL CHECK(amount>0 AND amount<=999999999999.99),
 paid_on date NOT NULL, method text NOT NULL CHECK(method IN ('bank_transfer','cash')),
 note text NOT NULL CHECK(length(note)<=500), actor_id bigint NOT NULL REFERENCES app_user(id),
 created_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(company_id,invoice_id,id),
 FOREIGN KEY(company_id,invoice_id) REFERENCES invoice_draft(company_id,id)
);
CREATE TABLE invoice_payment_reversal (
 company_id bigint NOT NULL, invoice_id bigint NOT NULL, payment_id bigint NOT NULL,
 reason text NOT NULL CHECK(length(trim(reason)) BETWEEN 1 AND 500),
 actor_id bigint NOT NULL REFERENCES app_user(id), created_at timestamptz NOT NULL DEFAULT now(),
 replacement_id bigint,
 PRIMARY KEY(company_id,invoice_id,payment_id),
 FOREIGN KEY(company_id,invoice_id,payment_id) REFERENCES invoice_payment(company_id,invoice_id,id),
 FOREIGN KEY(company_id,invoice_id,replacement_id) REFERENCES invoice_payment(company_id,invoice_id,id)
);
CREATE TABLE invoice_payment_command (
 company_id bigint NOT NULL, request_id uuid NOT NULL, invoice_id bigint NOT NULL,
 actor_id bigint NOT NULL REFERENCES app_user(id), input jsonb NOT NULL, result jsonb NOT NULL,
 created_at timestamptz NOT NULL DEFAULT now(), PRIMARY KEY(company_id,request_id),
 FOREIGN KEY(company_id,invoice_id) REFERENCES invoice_draft(company_id,id)
);
CREATE FUNCTION protect_payment_history() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'payment history is append-only'; END $$;
CREATE TRIGGER payment_immutable BEFORE UPDATE OR DELETE ON invoice_payment FOR EACH ROW EXECUTE FUNCTION protect_payment_history();
CREATE TRIGGER reversal_immutable BEFORE UPDATE OR DELETE ON invoice_payment_reversal FOR EACH ROW EXECUTE FUNCTION protect_payment_history();
CREATE TRIGGER payment_command_immutable BEFORE UPDATE OR DELETE ON invoice_payment_command FOR EACH ROW EXECUTE FUNCTION protect_payment_history();
CREATE INDEX invoice_payment_lookup ON invoice_payment(company_id,invoice_id,paid_on,id);
-- All money arithmetic stays NUMERIC; each invoice is one CZK receivable.
CREATE VIEW invoice_settlement AS
SELECT x.*,
 CASE WHEN paid>payable THEN 'overpaid' WHEN paid=payable THEN 'paid' WHEN paid=0 THEN 'unpaid' ELSE 'partial' END AS payment_status,
 GREATEST(payable-paid,0)::numeric(30,2) AS outstanding,
 GREATEST(paid-payable,0)::numeric(30,2) AS overpayment,
 (paid<payable AND due_on<today) AS overdue
FROM (
 SELECT d.company_id,d.id AS invoice_id,(d.totals->>'payable')::numeric(30,2) AS payable,
 (d.data->>'due_on')::date AS due_on,
 (now() AT TIME ZONE (c.settings->>'timezone'))::date AS today,
 COALESCE((SELECT sum(p.amount) FROM invoice_payment p WHERE p.company_id=d.company_id AND p.invoice_id=d.id
 AND NOT EXISTS(SELECT 1 FROM invoice_payment_reversal r WHERE r.company_id=p.company_id AND r.invoice_id=p.invoice_id AND r.payment_id=p.id)),0)::numeric(30,2) AS paid,
 ((SELECT count(*) FROM invoice_payment p WHERE p.company_id=d.company_id AND p.invoice_id=d.id)+
 (SELECT count(*) FROM invoice_payment_reversal r WHERE r.company_id=d.company_id AND r.invoice_id=d.id)) AS payment_revision
 FROM invoice_draft d JOIN company c ON c.id=d.company_id WHERE d.number IS NOT NULL
) x;
CREATE VIEW invoice_with_settlement AS
SELECT d.*, CASE WHEN s.invoice_id IS NULL THEN NULL ELSE jsonb_build_object(
 'paid',s.paid::text,'outstanding',s.outstanding::text,'overpayment',s.overpayment::text,
 'status',s.payment_status,'overdue',s.overdue,'revision',s.payment_revision,'today',s.today::text) END AS settlement
FROM invoice_draft d LEFT JOIN invoice_settlement s ON s.company_id=d.company_id AND s.invoice_id=d.id;
