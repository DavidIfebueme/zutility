# ZEC subscriptions research

Status: research note, not shipped. Last updated 2026-11-15.

## The proposal

Strahn Cryptography announced shielded subscriptions on Zcash, built on ELSA (Efficient Layer-2 Schnorr Apparition). A merchant and a user form a mandate over a shielded address that both control through a FROST key generation. Terms are fixed before any money arrives, and charges happen through pre-signed transactions rather than a new script. A mandate payment is indistinguishable from a normal shielded transfer on chain.

The authorisation model is credit-card shaped. The user cancels in the wallet, pre-funds several cycles, skips a cycle, or hands a subscription key to a watchtower that can only charge on the agreed terms. Funds cannot be stolen from the mandate.

## Current status

Not live on mainnet. The announcement says it needs ratification of a governance proposal to activate ZIP 68 (relative timelocks) and clarify ZIP 203 (absolute locktime for shielded transactions). Without those changes it still works, but the existence and schedule of a subscription are public and separate payments can be linked. Amounts, parties, and balances stay encrypted.

The demo runs on a special Arctic testnet with Arctic Wallet and a borrowed Medusa storefront that uses the stock Stripe plugin through a payment service provider layer.

The governance thread is open and the poll is small. ZIP 68 is not ratified. The last exchange in the thread is a Zcash core developer proposing a sighash variant instead of a consensus branch ID. There is no public merchant SDK or repository yet. The retroactive grant application states the code will be published during the review period.

## Sources

- Announcement: https://forum.zcashcommunity.com/t/trustless-self-custodial-shielded-subscriptions-built-natively-into-zcash/59263
- Governance proposal: https://forum.zcashcommunity.com/t/activate-zip-68-and-clarify-zip-203-to-facilitate-zcash-lightning-and-programmable-locks/58841
- Retroactive grant: https://forum.zcashcommunity.com/t/retroactive-grant-application-elsa-recurring-subscriptions-support-for-zcash-and-zip-research-and-development-with-stripe-layer/59296
- VCL paper: https://swiss.strahn.dev/p/2026/0002
- ELSA paper: https://swiss.strahn.dev/p/2026/0003
- Demo wallet: https://arctic-wallet.strahn.dev
- Demo merchant: https://arctic.strahn.dev

## The constraint for zutility

ELSA is a mandate between the user's wallet and the merchant. Zutility runs a zingolib wallet and hands out deposit addresses. Users pay from whatever wallet they hold. We are not their signer, so we cannot create a mandate for them.

That means zutility can only be the merchant side, and the user must bring an ELSA-capable wallet. Today that means Arctic. This is the ceiling on any demo before wallets adopt the protocol.

The second constraint is that the merchant integration is unpublished. The first step is a conversation with Strahn, not a sprint.

## Scope and limits

The protocol is not limited to utilities. Any merchant that adopts ELSA can request a mandate. Streaming, gym memberships, rent, school fees, hosting, and SaaS all fit the same model. Three things do not.

- A merchant that only accepts cards. ELSA is not a virtual card and cannot debit a card network. The merchant has to run the protocol.
- A variable amount with no cap. Terms are fixed before funding, and a merchant can only charge within them. A postpaid electricity bill needs a range or a fresh approval each cycle.
- A merchant that has not integrated. Netflix is the example. It works only if Netflix adopts ELSA, or if zutility resells a Netflix voucher and the mandate pays zutility for that voucher. Voucher resale is a different product with supply and redemption risk.

The other limit is privacy. Without ZIP 68 and the ZIP 203 fix, the existence and schedule of a subscription are public and payments can be linked.

## What the surface looks like

For the user, a Subscribe option next to the normal pay flow.

- Plans: MTN data monthly, DSTV monthly, a monthly electricity amount.
- Terms: amount, cycle, duration, cancellation, refunds.
- One approval in their wallet.
- Dashboard: next debit, cycles left, mandate status, cancel, receipts.

Each cycle:

1. The scheduler claims due cycles with compare-and-set, so one worker wins.
2. It asks the mandate to charge, with an idempotency key of mandate id plus cycle number.
3. The debit txid is recorded in `payments` and creates one order with a subscription source.
4. The existing Inlomax or VTPass dispatch path delivers the utility.
5. On failure: one retry, then refund through the mandate, then past due after N failures.

For the merchant: a mandate list, a per-cycle charge view, refunds, cancellations, and a webhook feed.

## What we would build

- `mandates` and `mandate_cycles` tables. Reuse `orders` and `payments`. A subscription cycle is an order with a nullable `mandate_cycle_id`, not a second pipeline.
- A `MandateProvider` trait beside `integrations/utility_provider.rs`, with create, charge, refund, cancel, and status. An `elsa` adapter selected by config, the way `ZCASH_BACKEND` selects the wallet.
- A 60 second scheduler job in `src/runtime/mod.rs`, following the existing tickers. Claim a cycle, charge, confirm, create the order, dispatch.
- Reconciliation by debit txid. Never credit by timestamp or amount alone.
- Refunds through the mandate when dispatch fails after a debit. If the protocol cannot refund yet, flag for review and show it to the user.
- Explicit failure policy: one retry, past due after N failures, cancel after M.
- Compliance copy on the subscribe screen: recurring amount, cycle, cancellation, refunds.

## How to be a better demo than Arctic

Do not compete on the wallet. Compete on the use case and the lifecycle.

- A real recurring bill instead of a borrowed clothing store.
- The full loop in one screen: mandate, debit, delivered utility, receipt, cancel, refund.
- Failure made visible: a declined cycle, a retry, a refund.
- A catalogue and naira pricing.
- Merchant metrics: collected, failed, refunded, time to delivery.
- An integration surface other merchants can use. Zutility as reference merchant and PSP adapter is a stronger position than another wallet.

A joint demo also helps Strahn. Their grant is pending and they need a second, real merchant.

## Phases

1. Talk to Strahn. Get the merchant spec, testnet access, and the code.
2. Testnet demo, four to six weeks. One plan, signed with Arctic Wallet, mandates, cycles, scheduler, dispatch on testnet, dashboard, cancel and refund.
3. Mainnet readiness after ZIP 68 and the ZIP 203 fix. Audit the adapter, switch networks, write the support runbook.
4. Platform. A merchant API and webhook for other Nigerian merchants.

## Bridge before ELSA

A saved plan with a reminder and a one-tap pay link, or a pre-funded zutility balance debited each cycle. Both are custodial and neither is trustless. Label them clearly. They prove demand and produce the subscription data that makes the ELSA case concrete.
