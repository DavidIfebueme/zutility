CREATE TABLE IF NOT EXISTS payments (
    id BIGSERIAL PRIMARY KEY,
    txid TEXT NOT NULL,
    order_id UUID NOT NULL REFERENCES orders(id) ON DELETE RESTRICT,
    address TEXT NOT NULL,
    amount_zec NUMERIC(18, 8) NOT NULL CHECK (amount_zec > 0),
    note_index INT NOT NULL DEFAULT 0 CHECK (note_index >= 0),
    confirmations INT NOT NULL DEFAULT 0 CHECK (confirmations >= 0),
    status TEXT NOT NULL DEFAULT 'seen' CHECK (status IN ('seen', 'dropped')),
    first_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (txid, address, amount_zec, note_index)
);

CREATE INDEX IF NOT EXISTS idx_payments_order_id ON payments (order_id);
CREATE INDEX IF NOT EXISTS idx_payments_txid ON payments (txid);
