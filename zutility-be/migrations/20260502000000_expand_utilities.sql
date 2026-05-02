ALTER TABLE utilities DROP CONSTRAINT IF EXISTS utilities_utility_type_check;
ALTER TABLE utilities ADD CONSTRAINT utilities_utility_type_check
  CHECK (utility_type IN ('airtime', 'data', 'dstv', 'gotv', 'startimes', 'showmax', 'electricity', 'school_fees', 'waec', 'jamb'));

ALTER TABLE orders DROP CONSTRAINT IF EXISTS orders_utility_type_check;
ALTER TABLE orders ADD CONSTRAINT orders_utility_type_check
  CHECK (utility_type IN ('airtime', 'data', 'dstv', 'gotv', 'startimes', 'showmax', 'electricity', 'school_fees', 'waec', 'jamb'));

ALTER TABLE orders ADD COLUMN IF NOT EXISTS variation_code TEXT;
ALTER TABLE orders ADD COLUMN IF NOT EXISTS provider TEXT;
ALTER TABLE orders ADD COLUMN IF NOT EXISTS customer_name TEXT;
