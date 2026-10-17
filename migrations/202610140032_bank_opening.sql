ALTER TABLE bank_account ADD COLUMN opening_on date;
ALTER TABLE bank_account ADD COLUMN opening_balance numeric(14,2);
ALTER TABLE bank_account ADD CONSTRAINT bank_opening_complete CHECK ((opening_on IS NULL) = (opening_balance IS NULL));
