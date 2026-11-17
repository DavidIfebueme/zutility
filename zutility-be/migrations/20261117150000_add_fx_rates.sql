CREATE TABLE fx_rates (
    quote CHAR(3) PRIMARY KEY REFERENCES currencies(code),
    per_usd NUMERIC(20, 10) NOT NULL CHECK (per_usd > 0),
    source TEXT NOT NULL,
    fetched_at TIMESTAMPTZ NOT NULL
);
