-- Only new DDPP/credit issues capture the active style; historical PDFs stay intact.
ALTER TABLE tax_receipt ADD COLUMN document_style jsonb;
ALTER TABLE credit_note ADD COLUMN document_style jsonb;
