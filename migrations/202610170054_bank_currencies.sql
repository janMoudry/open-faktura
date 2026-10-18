-- Native account/movement amounts; never reinterpret historical CZK amounts.
ALTER TABLE bank_account DROP CONSTRAINT bank_account_currency_check;
ALTER TABLE bank_account ADD CONSTRAINT bank_account_currency_check
 CHECK (currency IN ('CZK','EUR','USD','GBP','CHF','PLN','HUF','JPY','SEK','NOK','DKK','CAD','AUD'));
ALTER TABLE bank_account ADD CONSTRAINT bank_account_currency_identity UNIQUE(company_id,id,currency);
ALTER TABLE bank_movement ADD COLUMN currency text GENERATED ALWAYS AS (data->>'currency') STORED NOT NULL;
ALTER TABLE bank_movement ADD CONSTRAINT bank_movement_currency_account
 FOREIGN KEY(company_id,account_id,currency) REFERENCES bank_account(company_id,id,currency);
