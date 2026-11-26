CREATE TABLE countries (
    iso2 CHAR(2) PRIMARY KEY,
    iso3 CHAR(3) UNIQUE NOT NULL,
    name TEXT NOT NULL,
    currency CHAR(3) NOT NULL REFERENCES currencies(code),
    dial_code TEXT NOT NULL,
    timezone TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT false
);

CREATE TABLE provider_products (
    id UUID PRIMARY KEY,
    provider TEXT NOT NULL,
    failover_provider TEXT,
    provider_product_id TEXT NOT NULL,
    country_iso2 CHAR(2) NOT NULL REFERENCES countries(iso2),
    utility_type TEXT NOT NULL,
    utility_category TEXT NOT NULL,
    slug TEXT NOT NULL,
    name TEXT NOT NULL,
    descriptor TEXT NOT NULL,
    icon_key TEXT NOT NULL,
    service_ref_label TEXT NOT NULL,
    service_ref_placeholder TEXT NOT NULL,
    service_ref_pattern TEXT,
    quick_amounts JSONB,
    amount_kind TEXT NOT NULL CHECK (amount_kind IN ('fixed', 'ranged', 'variation')),
    amount_min NUMERIC(18, 4),
    amount_max NUMERIC(18, 4),
    amount_increment NUMERIC(18, 4),
    destination_currency CHAR(3) NOT NULL REFERENCES currencies(code),
    fulfilment_kind TEXT NOT NULL CHECK (fulfilment_kind IN ('pin', 'payment')),
    wholesale_rate NUMERIC(20, 10) NOT NULL,
    wholesale_fee NUMERIC(18, 4) NOT NULL DEFAULT 0,
    retail_rate NUMERIC(20, 10),
    retail_fee NUMERIC(18, 4),
    required_fields JSONB NOT NULL DEFAULT '{}',
    field_labels JSONB NOT NULL DEFAULT '{}',
    operator_id TEXT,
    operator_name TEXT,
    enabled BOOLEAN NOT NULL DEFAULT false,
    synced_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (amount_kind = 'variation' OR (amount_min IS NOT NULL AND amount_max IS NOT NULL)),
    CHECK (amount_kind <> 'fixed' OR amount_min = amount_max),
    CHECK (retail_rate IS NULL OR retail_rate > 0),
    UNIQUE (provider, provider_product_id),
    UNIQUE (country_iso2, slug)
);

CREATE INDEX idx_provider_products_country_category
    ON provider_products (country_iso2, utility_category, name);

CREATE TABLE provider_product_variations (
    id UUID PRIMARY KEY,
    product_id UUID NOT NULL REFERENCES provider_products(id) ON DELETE RESTRICT,
    provider_product_id TEXT NOT NULL,
    variation_code TEXT NOT NULL,
    name TEXT NOT NULL,
    amount NUMERIC(18, 4) NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT true,
    UNIQUE (product_id, variation_code)
);
