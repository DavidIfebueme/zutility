CREATE TABLE IF NOT EXISTS order_dispatch_attempts (
    id BIGSERIAL PRIMARY KEY,
    order_id UUID NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    attempt_no INT NOT NULL CHECK (attempt_no > 0),
    status TEXT NOT NULL CHECK (status IN ('started', 'success', 'retryable_failure', 'terminal_failure')),
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(order_id, attempt_no)
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_orders_vtpass_request_id
    ON orders(vtpass_request_id)
    WHERE vtpass_request_id IS NOT NULL;
