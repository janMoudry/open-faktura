ALTER TABLE import_batch DROP CONSTRAINT import_batch_source_check;
ALTER TABLE import_batch ADD CHECK(source IN ('idoklad','fakturaonline','generic','money_s3'));
ALTER TABLE import_identity DROP CONSTRAINT import_identity_kind_check;
ALTER TABLE import_identity DROP CONSTRAINT import_identity_check;
ALTER TABLE import_identity ADD COLUMN catalog_id bigint;
ALTER TABLE import_identity ADD FOREIGN KEY(company_id,catalog_id) REFERENCES catalog(company_id,id);
ALTER TABLE import_identity ADD CHECK(kind IN ('contact','invoice','catalog'));
ALTER TABLE import_identity ADD CHECK(
 (kind='contact' AND contact_id IS NOT NULL AND invoice_id IS NULL AND catalog_id IS NULL) OR
 (kind='invoice' AND invoice_id IS NOT NULL AND contact_id IS NULL AND catalog_id IS NULL) OR
 (kind='catalog' AND catalog_id IS NOT NULL AND contact_id IS NULL AND invoice_id IS NULL));
