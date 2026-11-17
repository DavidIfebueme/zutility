ALTER TABLE orders
    ADD COLUMN IF NOT EXISTS country_iso2 CHAR(2) NOT NULL DEFAULT 'NG',
    ADD COLUMN IF NOT EXISTS product_id UUID REFERENCES provider_products(id) ON DELETE RESTRICT,
    ADD COLUMN IF NOT EXISTS provider_external_id TEXT,
    ADD COLUMN IF NOT EXISTS provider_transaction_id TEXT,
    ADD COLUMN IF NOT EXISTS provider_status TEXT,
    ADD COLUMN IF NOT EXISTS failure_code TEXT,
    ADD COLUMN IF NOT EXISTS failure_message TEXT,
    ADD COLUMN IF NOT EXISTS settlement_currency CHAR(3),
    ADD COLUMN IF NOT EXISTS settlement_amount NUMERIC(18,4),
    ADD COLUMN IF NOT EXISTS cost_currency CHAR(3),
    ADD COLUMN IF NOT EXISTS cost_amount NUMERIC(18,4),
    ADD COLUMN IF NOT EXISTS inputs JSONB NOT NULL DEFAULT '{}',
    ADD COLUMN IF NOT EXISTS fulfilment_kind TEXT,
    ADD COLUMN IF NOT EXISTS flag_reason TEXT;

UPDATE orders
SET settlement_currency = COALESCE(settlement_currency, 'NGN'),
    settlement_amount = COALESCE(settlement_amount, amount_ngn),
    cost_currency = COALESCE(cost_currency, 'NGN'),
    cost_amount = COALESCE(cost_amount, amount_ngn);

UPDATE orders AS o
SET flag_reason = latest.event
FROM (
    SELECT DISTINCT ON (order_id) order_id, event
    FROM audit_log
    WHERE new_status = 'flagged_for_review'
    ORDER BY order_id, created_at DESC
) AS latest
WHERE o.id = latest.order_id
  AND o.status = 'flagged_for_review'
  AND o.flag_reason IS NULL;

CREATE INDEX IF NOT EXISTS idx_orders_provider_external_id ON orders (provider_external_id);
CREATE INDEX IF NOT EXISTS idx_orders_product_id ON orders (product_id);
