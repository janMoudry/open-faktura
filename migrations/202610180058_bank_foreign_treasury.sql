-- Extend existing foreign snapshots to explicit advance/refund targets; preserve history.
ALTER TABLE bank_match DROP CONSTRAINT bank_match_foreign_shape;
ALTER TABLE bank_match ADD CONSTRAINT bank_match_foreign_shape CHECK (
    foreign_settlement IS NULL OR ((
        jsonb_typeof(foreign_settlement) = 'object'
        AND (invoice_id IS NOT NULL OR received_id IS NOT NULL OR advance_id IS NOT NULL OR credit_id IS NOT NULL)
        AND foreign_settlement ?& ARRAY['currency','document_amount','document_rate','book_value_czk','difference_czk']
        AND (foreign_settlement->>'document_amount')::numeric > 0
        AND CASE WHEN foreign_settlement ? 'bank_currency' THEN
            foreign_settlement ?& ARRAY['bank_rate','bank_value_czk']
            AND foreign_settlement->>'bank_currency' <> foreign_settlement->>'currency'
            AND (foreign_settlement->>'bank_value_czk')::numeric >= 0
            AND CASE WHEN foreign_settlement->>'bank_currency' = 'CZK' THEN
                foreign_settlement->'bank_rate' = 'null'::jsonb
            ELSE
                jsonb_typeof(foreign_settlement->'bank_rate') = 'object'
                AND (foreign_settlement->'bank_rate'->>'rate')::numeric > 0
                AND (foreign_settlement->'bank_rate'->>'units')::numeric > 0
            END
        ELSE foreign_settlement->>'currency' <> 'CZK' END
    ) IS TRUE)
);
