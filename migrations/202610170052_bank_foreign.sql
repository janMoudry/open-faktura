-- Existing allocations remain in CZK; an optional immutable snapshot records
-- the amount settled in the foreign document currency. Stop old API before upgrade.
ALTER TABLE bank_match ADD COLUMN foreign_settlement jsonb;
ALTER TABLE bank_match ADD CONSTRAINT bank_match_foreign_shape CHECK (
    foreign_settlement IS NULL OR (
        jsonb_typeof(foreign_settlement) = 'object'
        AND created_payment
        AND (invoice_id IS NOT NULL OR received_id IS NOT NULL)
        AND foreign_settlement ?& ARRAY['currency','document_amount','document_rate','book_value_czk','difference_czk']
        AND foreign_settlement->>'currency' <> 'CZK'
        AND (foreign_settlement->>'document_amount')::numeric > 0
    )
);
