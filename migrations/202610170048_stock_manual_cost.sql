-- Preserve historical unknown costs; allow explicit positive acquisition snapshots.
ALTER TABLE stock_movement DROP CONSTRAINT stock_acquisition_cost_check;
ALTER TABLE stock_movement ADD CONSTRAINT stock_acquisition_cost_check CHECK (
 acquisition_unit_cost IS NULL OR
 (acquisition_unit_cost>=0 AND acquisition_unit_cost<=1000000000 AND
 (kind='received' OR (kind IN ('opening','manual','count') AND quantity>0)))
);
