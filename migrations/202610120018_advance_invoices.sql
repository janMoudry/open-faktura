-- P13-01: independent proforma series and receipts, outside invoice revenue.
ALTER TABLE sales_document DROP CONSTRAINT sales_document_kind_check;
ALTER TABLE sales_document ADD CONSTRAINT sales_document_kind_check CHECK(kind IN ('template','quote','advance'));
ALTER TABLE sales_document ADD COLUMN advance_totals jsonb;
ALTER TABLE sales_document ADD COLUMN advance_pdf bytea;
ALTER TABLE sales_document ADD COLUMN advance_paid numeric(20,2) NOT NULL DEFAULT 0 CHECK(advance_paid>=0);
CREATE TABLE advance_sequence(company_id bigint NOT NULL REFERENCES company(id),year integer NOT NULL,next_value bigint NOT NULL CHECK(next_value>0),PRIMARY KEY(company_id,year));
CREATE TABLE advance_payment(
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 company_id bigint NOT NULL, document_id bigint NOT NULL,
 actor_id bigint NOT NULL REFERENCES app_user(id), request_id uuid NOT NULL,
 command jsonb NOT NULL, amount numeric(20,2) NOT NULL CHECK(amount>0 AND amount<=999999999999.99),
 paid_on date NOT NULL,method text NOT NULL CHECK(method IN ('cash','bank_transfer')),
 note text NOT NULL,created_at timestamptz NOT NULL DEFAULT now(),
 reversed_at timestamptz, reversed_by bigint REFERENCES app_user(id),reason text,
 UNIQUE(company_id,request_id),UNIQUE(company_id,document_id,id),
 FOREIGN KEY(company_id,document_id) REFERENCES sales_document(company_id,id)
);
CREATE INDEX advance_payment_document ON advance_payment(company_id,document_id,id);
