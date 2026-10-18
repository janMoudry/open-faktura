-- Permit a foreign snapshot for an existing payment; retain all other shape checks.
ALTER TABLE bank_match DROP CONSTRAINT bank_match_foreign_shape;
ALTER TABLE bank_match ADD CONSTRAINT bank_match_foreign_shape CHECK (
    foreign_settlement IS NULL OR (
        jsonb_typeof(foreign_settlement) = 'object'
        AND (invoice_id IS NOT NULL OR received_id IS NOT NULL)
        AND foreign_settlement ?& ARRAY['currency','document_amount','document_rate','book_value_czk','difference_czk']
        AND foreign_settlement->>'currency' <> 'CZK'
        AND (foreign_settlement->>'document_amount')::numeric > 0
    )
);
