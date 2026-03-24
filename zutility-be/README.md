# zutility-be

Rust backend for zutility.

## Prerequisites

- Rust stable
- Docker (for local PostgreSQL)
- Local testnet-capable `zcashd` endpoint
- VTpass sandbox credentials

## Local dev stack

```bash
docker compose -f docker-compose.dev.yml up -d
cp .env.example .env
```

Update `.env` with real values for Zcash RPC and VTpass sandbox keys.

## Database setup

```bash
cargo install sqlx-cli --no-default-features --features postgres
sqlx database create
sqlx migrate run
```

## Run backend

```bash
cargo run
```

`cargo run` starts HTTP and background workers in one process.

## Test

```bash
cargo test
```

## Full local E2E

See `LOCAL_E2E.md`.
