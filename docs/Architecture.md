# architecture

## backend (zutility-be)

rust service built on axum. handles api routing, order lifecycle, payment detection, and provider dispatch.

- **api layer** — axum handlers with jwt auth (access + refresh + csrf), admin middleware, cors
- **order engine** — 60-second orchestrator loop: sync wallet → detect payments → dispatch utility purchase → update status
- **zcash integration** — zingolib light client (primary) or zcashd rpc (fallback). sync, balance, address generation, payment detection
- **provider dispatch** — inlomax (primary for airtime/data/tv/electricity/education), vtpass (fallback on outage), remita (school fees only)
- **rates** — coingecko/binance/kraken/coinbase for zec, with african fx rates (kes, ghs, zar, egp)
- **notifications** — per-user, created at order status transitions, polled every 30s
- **db** — postgresql 16 in docker, sqlx for queries

## frontend (zutility-fe)

next.js 14 / typescript / tailwind. app router with route groups.

- **(marketing)** — landing, how-it-works, support, waitlist
- **(auth)** — login, signup, forgot-password, reset-password, verify
- **(app)** — dashboard, pay flow, order detail, history, settings, otc (coming soon), p2p (coming soon)
- **state** — zustand with persist (auth, currency preference)
- **icons** — custom itshover animated icons (motion/react), brand icons for all nigerian utilities
- **middleware** — route protection via csrf_token cookie check

## communication

rest api. frontend hits backend via nginx reverse proxy. auth via http-only cookies (access_token, refresh_token, csrf_token).

## infrastructure

- linode vps (ubuntu 24.04)
- nginx + let's encrypt (https)
- systemd for backend service
- postgresql 16 in docker
- zcashd v6.1.0 (testnet full node)
- frontend deployed on vercel
