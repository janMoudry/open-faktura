CREATE TABLE credit_note_sequence(company_id bigint NOT NULL REFERENCES company(id),year integer NOT NULL,next_value bigint NOT NULL CHECK(next_value>0),PRIMARY KEY(company_id,year));
CREATE TABLE credit_note(
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,company_id bigint NOT NULL,invoice_id bigint NOT NULL,
 number text,status text NOT NULL DEFAULT 'draft' CHECK(status IN ('draft','issued','cancelled')),revision bigint NOT NULL DEFAULT 1,
 source jsonb NOT NULL,data jsonb NOT NULL,totals jsonb NOT NULL,pdf bytea,
 created_at timestamptz NOT NULL DEFAULT now(),updated_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(company_id,id),UNIQUE(company_id,id,invoice_id),UNIQUE(company_id,number),
 FOREIGN KEY(company_id,invoice_id) REFERENCES invoice_draft(company_id,id),
 CHECK((status='issued')=(number IS NOT NULL))
);
CREATE INDEX credit_note_source ON credit_note(company_id,invoice_id,status);
CREATE TABLE credit_note_command(
 company_id bigint NOT NULL,request_id uuid NOT NULL,actor_id bigint NOT NULL REFERENCES app_user(id),
 command jsonb NOT NULL,credit_id bigint NOT NULL,PRIMARY KEY(company_id,request_id),
 FOREIGN KEY(company_id,credit_id) REFERENCES credit_note(company_id,id)
);
CREATE TABLE credit_note_event(
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,company_id bigint NOT NULL,credit_id bigint NOT NULL,invoice_id bigint NOT NULL,
 kind text NOT NULL CHECK(kind IN ('offset','refund','reversal')),amount numeric(30,2) NOT NULL CHECK(amount>0),date date NOT NULL,
 method text NOT NULL,reason text NOT NULL,reverses bigint,actor_id bigint NOT NULL REFERENCES app_user(id),created_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(company_id,credit_id,id),UNIQUE(company_id,credit_id,reverses),
 FOREIGN KEY(company_id,credit_id,invoice_id) REFERENCES credit_note(company_id,id,invoice_id),
 FOREIGN KEY(company_id,credit_id,reverses) REFERENCES credit_note_event(company_id,credit_id,id),
 CHECK((kind='reversal')=(reverses IS NOT NULL))
);
CREATE TABLE credit_note_audit(
 company_id bigint NOT NULL,credit_id bigint NOT NULL,revision bigint NOT NULL,actor_id bigint NOT NULL REFERENCES app_user(id),
 action text NOT NULL,snapshot jsonb NOT NULL,created_at timestamptz NOT NULL DEFAULT now(),PRIMARY KEY(company_id,credit_id,revision),
 FOREIGN KEY(company_id,credit_id) REFERENCES credit_note(company_id,id)
);
CREATE TRIGGER credit_event_immutable BEFORE UPDATE OR DELETE ON credit_note_event FOR EACH ROW EXECUTE FUNCTION protect_payment_history();
CREATE TRIGGER credit_audit_immutable BEFORE UPDATE OR DELETE ON credit_note_audit FOR EACH ROW EXECUTE FUNCTION protect_payment_history();
CREATE TRIGGER credit_command_immutable BEFORE UPDATE OR DELETE ON credit_note_command FOR EACH ROW EXECUTE FUNCTION protect_payment_history();
-- Retain column order for existing dependent views. Cash is separate from setoff.
CREATE OR REPLACE VIEW invoice_settlement AS
SELECT company_id,invoice_id,payable,due_on,today,paid,payment_revision,
 CASE WHEN paid>payable THEN 'overpaid' WHEN paid=payable THEN 'paid' WHEN paid=0 THEN 'unpaid' ELSE 'partial' END AS payment_status,
 GREATEST(payable-paid,0)::numeric(30,2) AS outstanding,GREATEST(paid-payable,0)::numeric(30,2) AS overpayment,
 (paid<payable AND due_on<today) AS overdue,cash_paid
FROM (
 SELECT x.*, (cash_paid+offset_amount)::numeric(30,2) AS paid FROM (
 SELECT d.company_id,d.id AS invoice_id,(d.totals->>'payable')::numeric(30,2) AS payable,(d.data->>'due_on')::date AS due_on,
 (now() AT TIME ZONE (c.settings->>'timezone'))::date AS today,
 COALESCE((SELECT sum(p.amount) FROM invoice_payment p WHERE p.company_id=d.company_id AND p.invoice_id=d.id AND NOT EXISTS(SELECT 1 FROM invoice_payment_reversal r WHERE r.company_id=p.company_id AND r.invoice_id=p.invoice_id AND r.payment_id=p.id)),0)::numeric(30,2) AS cash_paid,
 COALESCE((SELECT sum(e.amount) FROM credit_note_event e WHERE e.company_id=d.company_id AND e.invoice_id=d.id AND e.kind='offset' AND NOT EXISTS(SELECT 1 FROM credit_note_event r WHERE r.company_id=e.company_id AND r.credit_id=e.credit_id AND r.reverses=e.id)),0)::numeric(30,2) AS offset_amount,
 ((SELECT count(*) FROM invoice_payment p WHERE p.company_id=d.company_id AND p.invoice_id=d.id)+(SELECT count(*) FROM invoice_payment_reversal r WHERE r.company_id=d.company_id AND r.invoice_id=d.id)+(SELECT count(*) FROM credit_note_event e WHERE e.company_id=d.company_id AND e.invoice_id=d.id)) AS payment_revision
 FROM invoice_draft d JOIN company c ON c.id=d.company_id WHERE d.number IS NOT NULL
 ) x
) y;
