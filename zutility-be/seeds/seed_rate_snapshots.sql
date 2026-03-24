INSERT INTO rate_snapshots (
    zec_ngn,
    zec_usd,
    usd_ngn,
    coingecko_zec_ngn,
    binance_zec_usd,
    kraken_zec_usd,
    coinbase_zec_usd,
    sources_used
)
VALUES (
    150000.0000,
    100.0000,
    1500.0000,
    150000.0000,
    100.0000,
    99.8000,
    100.2000,
    ARRAY['coingecko', 'binance', 'kraken', 'coinbase']
);
