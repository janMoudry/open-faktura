ALTER TABLE bank_match ADD COLUMN amount numeric(14,2);
UPDATE bank_match b SET amount=abs(m.amount) FROM bank_movement m WHERE b.company_id=m.company_id AND b.movement_id=m.id;
ALTER TABLE bank_match ALTER COLUMN amount SET NOT NULL;
ALTER TABLE bank_match ADD CONSTRAINT bank_match_positive_amount CHECK(amount>0);
DROP INDEX bank_match_movement;
CREATE INDEX bank_match_active_movement ON bank_match(company_id,movement_id) WHERE detached_at IS NULL;
-- Preserve historical idempotency snapshots, including already detached links.
CREATE FUNCTION upgrade_bank_movement_036(v jsonb) RETURNS jsonb LANGUAGE plpgsql AS $$
DECLARE total numeric; used numeric; links jsonb;
BEGIN
 total:=abs((v->'data'->>'amount')::numeric);
 SELECT coalesce(jsonb_agg(x || jsonb_build_object('amount',to_char(total,'FM999999999999990.00'))),'[]'::jsonb),
 coalesce(sum(CASE WHEN x->>'detached_at' IS NULL THEN total ELSE 0 END),0)
 INTO links,used FROM jsonb_array_elements(v->'matches') x;
 RETURN v || jsonb_build_object('matches',links,'allocated',to_char(used,'FM999999999999990.00'),'remaining',to_char(total-used,'FM999999999999990.00'));
END $$;
UPDATE bank_command SET result=jsonb_set(result,'{value}',upgrade_bank_movement_036(result->'value')) WHERE result->>'kind'='movement';
UPDATE bank_command SET result=jsonb_set(result,'{value}',(SELECT coalesce(jsonb_agg(upgrade_bank_movement_036(x)),'[]'::jsonb) FROM jsonb_array_elements(result->'value') x)) WHERE result->>'kind'='batch';
DROP FUNCTION upgrade_bank_movement_036(jsonb);
