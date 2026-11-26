ALTER TABLE orders DROP CONSTRAINT IF EXISTS orders_status_check;
ALTER TABLE orders ADD CONSTRAINT orders_status_check CHECK (
    status IN (
        'awaiting_payment',
        'payment_detected',
        'payment_confirmed',
        'awaiting_claim',
        'utility_dispatching',
        'completed',
        'expired',
        'failed',
        'flagged_for_review',
        'cancelled',
        'reversed'
    )
);

CREATE INDEX IF NOT EXISTS idx_orders_flag_reason ON orders (flag_reason)
    WHERE status = 'flagged_for_review';
