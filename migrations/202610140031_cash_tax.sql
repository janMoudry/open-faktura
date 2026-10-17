-- Standalone item tax snapshot; old movements remain non-tax cash records.
ALTER TABLE cash_movement ADD COLUMN tax JSONB;
ALTER TABLE cash_movement ADD CONSTRAINT cash_tax_object CHECK (tax IS NULL OR jsonb_typeof(tax) = 'object');
ALTER TABLE cash_movement ADD CONSTRAINT cash_tax_manual CHECK (tax IS NULL OR NOT automatic);
