-- Immutable receipt price snapshots. Existing movements remain unvalued.
ALTER TABLE stock_movement ADD COLUMN acquisition_unit_cost numeric(18,4);
ALTER TABLE stock_movement ADD COLUMN acquisition_correction boolean NOT NULL DEFAULT false;
ALTER TABLE stock_movement ADD CONSTRAINT stock_acquisition_cost_check CHECK (
 acquisition_unit_cost IS NULL OR
 (kind='received' AND acquisition_unit_cost>=0 AND acquisition_unit_cost<=1000000000)
);
