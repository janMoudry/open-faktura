ALTER TABLE stock_movement
 ADD COLUMN retail_id bigint,
 ADD CONSTRAINT stock_retail_fk FOREIGN KEY(company_id,retail_id) REFERENCES retail_document(company_id,id),
 ADD COLUMN invoice_id bigint,
 ADD COLUMN received_id bigint,
 ADD COLUMN credit_id bigint,
 ADD COLUMN source_line text,
 ADD CONSTRAINT stock_invoice_fk FOREIGN KEY(company_id,invoice_id) REFERENCES invoice_draft(company_id,id),
 ADD CONSTRAINT stock_received_fk FOREIGN KEY(company_id,received_id) REFERENCES received_invoice(company_id,id),
 ADD CONSTRAINT stock_credit_fk FOREIGN KEY(company_id,credit_id) REFERENCES credit_note(company_id,id),
 ADD CONSTRAINT stock_source_check CHECK(num_nonnulls(invoice_id,received_id,credit_id,retail_id)<=1);
CREATE INDEX stock_invoice_source ON stock_movement(company_id,invoice_id) WHERE invoice_id IS NOT NULL;
CREATE INDEX stock_received_source ON stock_movement(company_id,received_id) WHERE received_id IS NOT NULL;
CREATE INDEX stock_credit_source ON stock_movement(company_id,credit_id) WHERE credit_id IS NOT NULL;
CREATE INDEX stock_retail_source ON stock_movement(company_id,retail_id) WHERE retail_id IS NOT NULL;
