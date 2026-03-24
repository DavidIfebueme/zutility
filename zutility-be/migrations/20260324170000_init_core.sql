CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE IF NOT EXISTS rate_snapshots (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    zec_ngn NUMERIC(18, 4) NOT NULL CHECK (zec_ngn > 0),
    zec_usd NUMERIC(18, 4) NOT NULL CHECK (zec_usd > 0),
    usd_ngn NUMERIC(18, 4) NOT NULL CHECK (usd_ngn > 0),
    coingecko_zec_ngn NUMERIC(18, 4),
    binance_zec_usd NUMERIC(18, 4),
    kraken_zec_usd NUMERIC(18, 4),
    coinbase_zec_usd NUMERIC(18, 4),
    sources_used TEXT[] NOT NULL DEFAULT '{}',
    fetched_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_rate_snapshots_fetched ON rate_snapshots (fetched_at DESC);

CREATE TABLE IF NOT EXISTS utilities (
    slug TEXT PRIMARY KEY,
    utility_type TEXT NOT NULL CHECK (utility_type IN ('airtime', 'data', 'dstv', 'gotv', 'electricity')),
    name TEXT NOT NULL,
    logo_url TEXT,
    field_config JSONB NOT NULL DEFAULT '{}',
    active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS deposit_addresses (
    address TEXT PRIMARY KEY,
    address_type TEXT NOT NULL CHECK (address_type IN ('shielded', 'transparent')),
    order_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    used BOOLEAN NOT NULL DEFAULT false
);

CREATE INDEX IF NOT EXISTS idx_deposit_addresses_unused_type
    ON deposit_addresses (address_type, used)
    WHERE used = false;

CREATE TABLE IF NOT EXISTS orders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    status TEXT NOT NULL DEFAULT 'awaiting_payment' CHECK (
        status IN (
            'awaiting_payment',
            'payment_detected',
            'payment_confirmed',
            'utility_dispatching',
            'completed',
            'expired',
            'failed',
            'flagged_for_review',
            'cancelled'
        )
    ),
    access_token_hash TEXT NOT NULL,
    utility_type TEXT NOT NULL CHECK (utility_type IN ('airtime', 'data', 'dstv', 'gotv', 'electricity')),
    utility_slug TEXT NOT NULL,
    service_ref TEXT NOT NULL,
    amount_ngn BIGINT NOT NULL CHECK (amount_ngn > 0),
    deposit_address TEXT NOT NULL UNIQUE REFERENCES deposit_addresses(address),
    address_type TEXT NOT NULL CHECK (address_type IN ('shielded', 'transparent')),
    zec_amount NUMERIC(18, 8) NOT NULL CHECK (zec_amount > 0),
    zec_rate_id UUID NOT NULL REFERENCES rate_snapshots(id),
    txid TEXT,
    confirmations INT NOT NULL DEFAULT 0 CHECK (confirmations >= 0),
    required_confs INT NOT NULL CHECK (required_confs > 0),
    total_received NUMERIC(18, 8),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    confirmed_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    vtpass_request_id TEXT,
    vtpass_response JSONB,
    delivery_token TEXT,
    ip_hash TEXT,
    metadata JSONB NOT NULL DEFAULT '{}',
    CONSTRAINT orders_expiry_after_create CHECK (expires_at > created_at),
    CONSTRAINT orders_total_received_non_negative CHECK (total_received IS NULL OR total_received >= 0)
);

ALTER TABLE deposit_addresses
    ADD CONSTRAINT fk_deposit_addresses_order
    FOREIGN KEY (order_id)
    REFERENCES orders(id)
    ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_orders_status ON orders(status);
CREATE INDEX IF NOT EXISTS idx_orders_deposit_addr ON orders(deposit_address);
CREATE INDEX IF NOT EXISTS idx_orders_expires_at ON orders(expires_at) WHERE status = 'awaiting_payment';
CREATE INDEX IF NOT EXISTS idx_orders_service_ref_created ON orders(service_ref, created_at DESC);

CREATE TABLE IF NOT EXISTS audit_log (
    id BIGSERIAL PRIMARY KEY,
    order_id UUID REFERENCES orders(id) ON DELETE SET NULL,
    event TEXT NOT NULL,
    old_status TEXT,
    new_status TEXT,
    detail JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_audit_log_order_id_created ON audit_log(order_id, created_at DESC);
