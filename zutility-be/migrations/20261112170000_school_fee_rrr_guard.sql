WITH ranked AS (
    SELECT id,
           ROW_NUMBER() OVER (
               PARTITION BY service_ref
               ORDER BY (total_received IS NOT NULL) DESC, created_at, id
           ) AS position
    FROM orders
    WHERE utility_type = 'school_fees'
      AND status NOT IN ('completed', 'failed', 'expired', 'cancelled')
),
cancelled AS (
    UPDATE orders
    SET status = 'cancelled'
    WHERE orders.id IN (SELECT id FROM ranked WHERE position > 1)
      AND orders.utility_type = 'school_fees'
      AND orders.status NOT IN ('completed', 'failed', 'expired', 'cancelled')
      AND orders.total_received IS NULL
    RETURNING orders.id, orders.service_ref
)
INSERT INTO audit_log (order_id, event, old_status, new_status, detail)
SELECT id, 'duplicate_rrr_cancelled', NULL, 'cancelled',
       jsonb_build_object('rrr', service_ref)
FROM cancelled;

CREATE UNIQUE INDEX IF NOT EXISTS ux_orders_active_school_fee_rrr
    ON orders (service_ref)
    WHERE utility_type = 'school_fees'
      AND status NOT IN ('completed', 'failed', 'expired', 'cancelled');
