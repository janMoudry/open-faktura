-- Read-only projection of stored sales lines, independent of stock tracking.
CREATE VIEW item_sales_line AS
WITH documents AS (
 SELECT company_id,id,number,'invoice'::text kind,(data->>'issued_on')::date issued_on,
 data#>'{calculation,lines}' lines,totals->'lines' amounts,totals->>'currency' currency,NULL::jsonb corrections
 FROM invoice_draft WHERE number IS NOT NULL
 UNION ALL
 SELECT company_id,id,number,'retail',(data#>>'{invoice,issued_on}')::date,
 data#>'{invoice,calculation,lines}',totals->'lines',totals->>'currency',NULL::jsonb
 FROM retail_document WHERE NOT cancelled
 UNION ALL
 SELECT company_id,id,number,'credit_note',(data->>'issued_on')::date,
 source#>'{data,calculation,lines}',totals->'lines',totals->>'currency',data->'lines'
 FROM credit_note WHERE status='issued'
)
SELECT d.company_id,d.id document_id,d.number,d.kind,d.issued_on,d.currency,
 c.id catalog_id,c.data->>'name' name,c.data->>'code' code,
 l->>'id' line_id,l->>'description' description,l->>'unit' unit,
 CASE WHEN d.kind='credit_note' THEN -coalesce((SELECT (x->>'return_quantity')::numeric FROM jsonb_array_elements(d.corrections) x WHERE x->>'id'=l->>'id'),0)
 ELSE (l->>'quantity')::numeric END quantity,
 (a#>>'{amounts,net}')::numeric net,(a#>>'{amounts,gross}')::numeric gross
FROM documents d
CROSS JOIN LATERAL jsonb_array_elements(d.lines) l
CROSS JOIN LATERAL jsonb_array_elements(d.amounts) a
JOIN catalog c ON c.company_id=d.company_id AND c.id::text=l->>'catalog_id'
WHERE a->>'id'=l->>'id';
