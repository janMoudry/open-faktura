-- Add native statement provenance; existing rows and IDs remain unchanged.
ALTER TABLE bank_movement DROP CONSTRAINT bank_movement_source_check;
ALTER TABLE bank_movement ADD CONSTRAINT bank_movement_source_check CHECK(source IN ('csv','manual','camt053','fio_json','moneta_xml','rb_abo'));
