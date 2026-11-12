CREATE UNIQUE INDEX IF NOT EXISTS ux_orders_active_school_fee_rrr
    ON orders (service_ref)
    WHERE utility_type = 'school_fees'
      AND status IN (
          'awaiting_payment',
          'payment_detected',
          'payment_confirmed',
          'utility_dispatching',
          'flagged_for_review'
      );
