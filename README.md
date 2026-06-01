# zutility

pay for everyday african utilities with zcash

## what is this

zutility lets you pay for airtime, data, cable tv, electricity, and education fees across nigeria using zec (zcash). currently on testnet. mainnet coming soon.

## tech stack

- **backend:** rust / axum / sqlx / postgresql
- **frontend:** next.js / typescript / tailwind
- **zcash:** zingolib (light client) + zcashd (full node fallback)
- **providers:** inlomax (primary), vtpass (fallback), remita (school fees)
- **infra:** linode / nginx / systemd

## project structure

```
zutility-be/    → rust backend (api, order engine, payment detection)
zutility-fe/    → next.js frontend (dashboard, pay flow, auth)
```

## getting started

check `setup.md` (gitignored) for full deployment instructions.

```bash
# backend
cd zutility-be
cp .env.example .env   # fill in your keys
cargo run

# frontend
cd zutility-fe
npm install
npm run dev
```

## status

work in progress. testnet only. registration is currently closed — join the waitlist.

## license

mit
