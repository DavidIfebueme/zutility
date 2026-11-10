# zutility agent rules

Global pstack rules in `~/.config/opencode/AGENTS.md` still apply. This file adds project rules. If they conflict, ask the user.

## Conversational style

- Keep answers short and direct.
- No emojis in commits, issues, PR comments, or code.
- No fluff or cheerful filler.
- Answer the question first, before edits or commands.
- When responding to feedback, say whether you agree or disagree before saying what changed.

## Repo shape

- `zutility-be` is the Rust backend. Axum, sqlx, Postgres, zingolib.
- `zutility-fe` is the Next.js frontend. App router, TypeScript, Tailwind, zustand.
- `docs/Architecture.md` and `docs/API-Reference.md` describe the system.
- `setup.md` is gitignored. Never commit secrets, `.env` files, keys, or seeds.

## Code quality

- Read a file in full before wide changes or before editing a file you have not inspected.
- No comments in code. No narrating comments, no TODO markers, no commented-out code. A comment is allowed only for a non-obvious why the code cannot express.
- No `any` in TypeScript. Decode external data into named types at the boundary.
- No inline imports. Top-level imports only.
- Inline single-line helpers with one call site.
- Do not preserve backward compatibility unless asked.
- Ask before removing functionality that looks intentional.
- Never downgrade a dependency to fix a type error. Upgrade it.
- Match existing patterns. Do not add a second style for the same job.
- Do not overengineer. Pick the simplest approach that fits the codebase.

## Rust

- Edition 2024. `unsafe` is denied. `unwrap`, `expect`, and `panic` are denied crate-wide, including tests. Return `Result` and use `?`.
- Use `thiserror` for domain errors and `anyhow` at the application boundary.
- Use `rust_decimal::Decimal` for money. Never use `f64` for NGN or ZEC.
- NGN is stored as integer naira. ZEC uses 8 decimal places.
- Never log secrets, seeds, private keys, JWTs, or full `DATABASE_URL`. Use `secrecy::SecretString`.
- Use `tracing` with structured fields. Never `println!` or `dbg!` in service code.
- Do not hold a `tokio::sync` lock across `.await` unless the lock is the point of the operation. Use a scoped lock.
- Do not block the async runtime. Use `spawn_blocking` for CPU or blocking I/O.
- Put validation at the boundary. Parse external payloads into typed values. Trust internal types.
- Make illegal states unrepresentable. Use enums and newtypes, not stringly checks.
- Use the order state machine in `domain/order.rs`. Do not add status transitions outside `can_transition_to`.
- Use compare-and-set updates for order state. Do not read, decide, and write in separate transactions.
- Add a new migration instead of editing an existing one. Migrations are append-only.
- Use `sqlx` parameter binding. Never build SQL with string interpolation.
- Verify webhook signatures and make provider callbacks idempotent.
- Never trust a client amount. Recompute prices and totals on the server.
- Add `utoipa` annotations when adding or changing an HTTP endpoint.
- Add a test for every state transition, threshold, and payment-detection edge case.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` before pushing.

## TypeScript and Next.js

- `npx tsc --noEmit` and `npm run lint` before pushing.
- Use server components by default. Add `"use client"` only when the component needs state, effects, or browser APIs.
- Keep API calls in `lib/api.ts` or a typed client. Do not scatter raw `fetch` calls.
- No `any`. Type API responses in `lib/types.ts`.
- Use `zod` at form and API boundaries.
- Do not add a second state library. Use the existing zustand stores.

## Commands

- Backend: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`.
- Frontend: `npx tsc --noEmit`, `npm run lint`.
- Do not run `cargo test` against a live database unless the test requires it. Use the existing test harness.
- Do not run `next build` unless requested.
- For ad-hoc scripts, write to `/tmp`, run, then remove.

## Dependencies

- Treat `Cargo.lock`, `package-lock.json`, and migration changes as reviewed code.
- Keep direct dependencies pinned. Use `cargo update -p <crate>` for a targeted bump.
- Run `cargo audit` and `npm audit` when adding a dependency.
- Do not add a dependency for something the standard library or an existing crate already does.

## Git

- Only commit files you changed in this session.
- Stage explicit paths. Never `git add -A` or `git add .`.
- Run `git status` before committing.
- Commit format: `feat(scope): ...`, `fix(scope): ...`, `docs(scope): ...`, `chore(scope): ...`.
- Never run `git reset --hard`, `git checkout .`, `git clean -fd`, `git stash`, or `git commit --no-verify`.
- Never force push.
- If a rebase conflict is in a file you did not modify, abort and ask.
- Do not commit plans or scratch files unless the user asks.
- Never commit secrets, keys, `.env`, or `setup.md`.

## Issues and PRs

- Open PRs ready, not draft, unless asked.
- One PR is one change with its own verification.
- Write PR bodies to a temp file and use `gh pr create --body-file`.
- Keep comments concise and technical. No emojis.
- Do not check out a PR branch unless the user asks. Inspect with `gh pr view`, `gh pr diff`, and `git show`.
- Include reproduction and verification evidence in the PR body.

## Zcash and NU7

- Mainnet and production testnet use `ZCASH_BACKEND=zingolib` against a lightwalletd or Zaino indexer.
- The zcashd RPC backend is legacy. It cannot handle Ironwood and must not be used on mainnet.
- `zingolib` is the wallet. It holds the seed, derives addresses, scans notes, and builds transactions.
- Payment detection must match the order's deposit address. Never credit an order from a timestamp-only wallet scan.
- Do not change `REQUIRED_CONFS_TRANSPARENT` or `REQUIRED_CONFS_SHIELDED` without stating the time and work tradeoff.
- The wallet seed is not a test fixture. Back it up offline and never log it.

## User override

If the user's instructions conflict with this file, ask for explicit confirmation before overriding. Then execute.
