# changelog

## [unreleased]

### added
- full auth system (register, login, refresh, logout, email verification, password reset)
- waitlist with verification
- 28 utilities across 6 categories (airtime, data, tv, electricity, education, school fees)
- inlomax integration with circuit breaker
- vtpass fallback provider
- remita integration for school fees
- zingolib light client for zcash wallet operations
- zcashd full node as fallback backend
- live zec rates from coingecko/binance/kraken/coinbase
- african fx rates (kes, ghs, zar, egp)
- currency preference system with timezone detection
- notifications system with real-time polling
- user settings (profile, security, preferences, danger zone)
- admin wallet balance endpoint
- custom brand icons for all nigerian utilities (mtn, airtel, glo, 9mobile, dstv, gotv, startimes, showmax, 12 discos, waec, jamb, school fees)
- payment flow animated hero (svg + motion/react)
- otc and p2p coming soon pages
- retry logic and health probes for zingolib indexer resilience
- dt one prepaid catalogue for eleven countries behind one product table
- `GET /api/v1/catalogue` with enabled countries, categories, products, and variations
- multi-country orders with settlement currency, provider dispatch, and reversals
- per-user display currency on every money response
- order settlement, fulfilment kind, and failure code on the status response
- country and product pickers on the pay page driven by the catalogue

### changed
- replaced zcashd with zingolib as primary zcash backend
- replaced 3d coin hero with payment flow diagram
- moved notification bell to sidebar header
- auth guard hardened with hydration flag
- cors fix for 429 responses (governor layer inside cors layer)
- pay page submits `product_id`, `settlement`, `inputs`, and `variation_code` instead of `amount_ngn`
- order page shows the settlement amount, the display amount, zec, failure code, and reversals
- marketing counts and faq read the catalogue instead of nigerian-only constants
- gifts and gift claims resolve the biller from the catalogue
- deleted the legacy `GET /api/v1/utilities` endpoint and its constants
- deleted the client-side naira conversion helpers; every amount comes from the server display block
