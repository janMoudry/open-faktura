ALTER TABLE received_series ADD COLUMN kind text NOT NULL DEFAULT 'invoice' CHECK(kind IN ('invoice','receipt'));
ALTER TABLE received_series DROP CONSTRAINT received_series_pkey;
ALTER TABLE received_series ADD PRIMARY KEY(company_id,year,kind);
ALTER TABLE received_invoice ADD COLUMN kind text NOT NULL DEFAULT 'invoice' CHECK(kind IN ('invoice','receipt'));
CREATE INDEX received_kind_register ON received_invoice(company_id,kind,received_on DESC,id DESC);
-- Existing JSON snapshots and replay records retain their original content;
-- serde defaults decode them as invoices. Stop the old server before upgrading.
