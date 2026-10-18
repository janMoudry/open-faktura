-- Document-owned rate: CZK per units of document currency. Numeric arithmetic only.
-- Existing CZK JSON snapshots need no rewrite.
CREATE FUNCTION report_czk(value numeric, currency text, exchange jsonb) RETURNS numeric
LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE rate numeric; units numeric;
BEGIN
 IF currency='CZK' THEN RETURN round(value,2); END IF;
 IF currency IS NULL OR currency NOT IN ('EUR','USD','GBP','CHF','PLN','HUF','JPY','SEK','NOK','DKK','CAD','AUD')
 OR exchange IS NULL OR NOT COALESCE(exchange->>'rate' ~ '^[0-9]+(\.[0-9]{1,6})?$',false)
 OR NOT COALESCE(exchange->>'units' ~ '^[0-9]+$',false) THEN
 RAISE EXCEPTION 'Missing or invalid document exchange rate' USING ERRCODE='22023'; END IF;
 rate:=(exchange->>'rate')::numeric; units:=(exchange->>'units')::numeric;
 IF rate<=0 OR rate>1000000 OR units<=0 OR units>1000000 THEN
 RAISE EXCEPTION 'Invalid document exchange rate bounds' USING ERRCODE='22023'; END IF;
 RETURN round(value*rate/units,2);
END $$;
