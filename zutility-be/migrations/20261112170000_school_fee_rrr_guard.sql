WITH ranked AS (
    SELECT id,
           ROW_NUMBER() OVER (PARTITION BY service_ref ORDER BY created_at, id) AS position
    FROM orders
    WHERE utility_type = 'school_fees'
      AND status NOT IN ('completed', 'failed', 'expired', 'cancelled')
      AND total_received IS NULL
)
UPDATE orders
SET status = 'cancelled'
FROM ranked
WHERE orders.id = ranked.id
  AND ranked.position > 1;

CREATE UNIQUE INDEX IF NOT EXISTS ux_orders_active_school_fee_rrr
    ON orders (service_ref)
    WHERE utility_type = 'school_fees'
      AND status NOT IN ('completed', 'failed', 'expired', 'cancelled');
