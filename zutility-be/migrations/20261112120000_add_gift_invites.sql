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
        'cancelled'
    )
);

CREATE TABLE IF NOT EXISTS gift_invites (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code_hash TEXT NOT NULL UNIQUE,
    sender_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    order_id UUID REFERENCES orders(id) ON DELETE SET NULL,
    utility_type TEXT NOT NULL,
    utility_slug TEXT NOT NULL,
    amount_ngn BIGINT NOT NULL CHECK (amount_ngn > 0),
    claim_window_hours INT NOT NULL CHECK (claim_window_hours BETWEEN 12 AND 168),
    message TEXT,
    sender_name TEXT,
    hide_sender_name BOOLEAN NOT NULL DEFAULT false,
    fallback_service_ref TEXT,
    status TEXT NOT NULL DEFAULT 'open' CHECK (
        status IN ('open', 'claimed', 'cancelled', 'expired')
    ),
    claimed_service_ref TEXT,
    claimed_customer_name TEXT,
    claimed_at TIMESTAMPTZ,
    claim_expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_gift_invites_order_id
    ON gift_invites (order_id)
    WHERE order_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_gift_invites_sender_created
    ON gift_invites (sender_user_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_gift_invites_open_claim_expiry
    ON gift_invites (claim_expires_at)
    WHERE status = 'open' AND claim_expires_at IS NOT NULL;
