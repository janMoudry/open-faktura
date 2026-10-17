ALTER TABLE stock_movement ADD COLUMN acquisition_quantity numeric(18,4);
ALTER TABLE stock_movement ADD COLUMN acquisition_extra_cost numeric(18,2) NOT NULL DEFAULT 0;
WITH quantities AS (
 SELECT id,sum(quantity) OVER(PARTITION BY company_id,catalog_id,received_id,source_line ORDER BY id) AS quantity
 FROM stock_movement WHERE received_id IS NOT NULL
)
UPDATE stock_movement m SET acquisition_quantity=q.quantity FROM quantities q WHERE q.id=m.id;
ALTER TABLE stock_movement ADD CONSTRAINT stock_extra_cost_check CHECK(acquisition_extra_cost>=0 AND acquisition_extra_cost<=1000000000);
ALTER TABLE stock_movement ADD CONSTRAINT stock_acquisition_quantity_check CHECK(acquisition_quantity IS NULL OR acquisition_quantity>=0);
