-- Existing documents keep NULL: never backfill current brand into history.
ALTER TABLE sales_document ADD COLUMN document_style jsonb;
ALTER TABLE cash_movement ADD COLUMN document_style jsonb;
