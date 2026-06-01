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

### changed
- replaced zcashd with zingolib as primary zcash backend
- replaced 3d coin hero with payment flow diagram
- moved notification bell to sidebar header
- auth guard hardened with hydration flag
- cors fix for 429 responses (governor layer inside cors layer)
