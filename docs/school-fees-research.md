# Nigerian university fees with ZEC

Research note. Written 2026-11-11. No code in this document. The goal is to add school fee payments for UNN, UI, and other Nigerian universities without building a scraper per school.

## Summary

The schools do not expose a simple "pay fees" bank account. They generate a reference for a specific student and a specific fee. The student pays that reference at a bank, on Remita, or through a card link. The school reconciles by reference.

Remita's Collections API can validate and pay that reference. One integration covers every university that is a Remita biller. That is the clean path.

The product should start with the reference the student already has. The student generates the RRR on the school portal, then pays it in zutility with ZEC. A later phase can generate the RRR inside zutility for schools whose biller products expose the fee schedule.

## How the schools collect fees today

### University of Nigeria, Nsukka

The portal is `https://unnportal.unn.edu.ng/`, a React application built by Tenece. The flow is:

1. The student signs in and opens Academic Fees.
2. The portal generates an invoice for the fee, for example `/academic_fees/school_fees/generate_invoice`.
3. The invoice carries an **RRR** and a card payment link.
4. The student pays on Remita or by card.
5. The portal reconciles the payment against the RRR.

The portal bundle contains the exact labels "Remita Retrieval Reference #" and "Kindly ensure your payment is made with an RRR generated from the Portal", and links to `remita.net`.

### University of Ibadan

The acceptance fee notice sends candidates to `https://admissions.ui.edu.ng/`. The student portal is `http://student-portal.ui.edu.ng/login`, the bursary is `https://bursary.ui.edu.ng/`, and receipt checks are at `http://streceipt.ui.edu.ng/`.

The public notice does not name the payment rail. Most federal universities use Remita RRR, but this needs confirmation with the UI bursary before it is treated as fact.

### Other universities

Federal universities generally use Remita under the Treasury Single Account model. State universities are mixed and may use Remita, Interswitch, or a private portal. The authoritative list is Remita's biller directory, not a guess per school.

### What this means

The school owns the fee schedule and the reference. A third party does not need the student's portal password or matric number to pay a reference. It needs the RRR, which is already bound to the student and the amount.

## The clean rail

Remita's current **Collections** API covers bill lookup, validation, and payment. The endpoints below come from the official Remita Postman collection.

Base URL for the demo environment:

```
https://api-demo.systemspecsng.com/services/connect-gateway/api/v1
```

Authentication is a `secretKey` header issued through RemitaConnect. The demo keys work immediately after registration. Production keys require activation and a service subscription.

| Step | Method and path | Body | Purpose |
| --- | --- | --- | --- |
| 1 | `GET /biller/get-billers?page=0&size=2000` | none | Find the university biller |
| 2 | `GET /biller/{productId}/products` | none | List the biller's fee products |
| 3 | `POST /biller/validate-customer` | `{"billPaymentProductId":"...","customerId":"..."}` | Resolve the student name |
| 4 | `POST /biller/initiate` | `{"billPaymentProductId":"...","amount":2000,"name":"John Doe","paymentIdentifier":"...","email":"...","phoneNumber":"...","customerId":"..."}` | Create a transaction |
| 5 | `GET /biller/lookup/{rrr}` | none | Validate or query an RRR |
| 6 | `POST /biller/process` | `{"rrr":"200799091919","paymentIdentifier":"...","amount":10000}` | Pay an existing RRR |

The two calls that matter for the first phase are 5 and 6. `lookup` returns the biller, the amount, the payer or student name, and the status. `process` pays the reference with an idempotency key.

## Integration options

| Option | Flow | Effort | Risk |
| --- | --- | --- | --- |
| A. RRR first | Student generates the RRR on the school portal. zutility validates it and pays it. | Low. One Remita integration covers every Remita biller. | Two-app flow. The student visits the school portal first. |
| B. In-app RRR | zutility calls `get-billers`, `products`, `validate-customer`, then `initiate` to create the reference inside zutility. | Medium. Per-school product mapping and fee schedules. | Fee schedules vary by faculty, level, and session. A wrong product charges the wrong amount. |
| C. Direct portal | Integrate the Tenece UNN portal or the UI portal directly. | High. Requires school cooperation and ongoing maintenance. | Breaks on every portal release. |

Ship A first. It works across UNN, UI, UNILAG, OAU, UNIBEN, and the rest without a per-school contract. Add B for UNN and UI after their biller products are confirmed.

## The flow to build

```text
student [1]
├── school portal [1a]
│   └── generates RRR + amount [1a1]
└── zutility school fees [1b]
    ├── lookup RRR [1b1]  ← biller, student, amount, status
    ├── create ZEC order at the looked-up amount [1b2]
    ├── wait for confirmations [1b3]
    ├── process RRR [1b4]  ← paymentIdentifier = order id
    ├── requery RRR [1b5]
    └── receipt with RRR + Remita reference [1b6]
```

## What the current backend gets wrong

`zutility-be/src/integrations/remita/mod.rs` uses the legacy first-generation endpoints.

`RemitaClient::pay` posts to `billpayment/pay` and never sends the RRR in the payload. The RRR appears only in the description and the phone field. That cannot reliably pay a university reference. The dispatcher in `zutility-be/src/integrations/provider_dispatcher.rs` routes `school_fees` to this client, so school fees look wired but are not.

The fix is a new Collections client with `lookup` and `process`, used only for `school_fees`. Keep the legacy client for the electricity fallback until that path is migrated.

New configuration:

```
REMITA_COLLECTIONS_BASE_URL=https://api-demo.systemspecsng.com/services/connect-gateway/api/v1
REMITA_SECRET_KEY=...
```

The old `REMITA_MERCHANT_ID`, `REMITA_API_KEY`, and `REMITA_SERVICE_TYPE_ID` belong to the first-generation invoice flow and are not used by Collections.

## Order model

Store the reference data with the order so support can trace it without calling Remita first.

| Field | Example | Notes |
| --- | --- | --- |
| `rrr` | `200799091919` | Single use. Immutable once an order exists. |
| `biller_name` | `University of Nigeria, Nsukka` | From `lookup`. |
| `product_name` | `School fees` | From `lookup`. |
| `student_name` | `Ada Okafor` | From `lookup`. Shown before payment. |
| `fee_amount_ngn` | `84500` | From `lookup`. Never from the client. |
| `session` | `2026/2027` | If the lookup exposes it. |
| `remita_reference` | returned by `process` | Delivery token. |

## Guardrails

- Validate the RRR before creating an order. Show the biller, student name, amount, and status.
- Reject paid, expired, cancelled, or mismatched references.
- Use the looked-up amount, never a client amount.
- One RRR per order. Store the RRR and the Remita reference.
- On any retry, call `lookup` first. Never call `process` twice for the same order.
- If `process` fails after ZEC is received, move the order to review. Do not retry blindly.
- Refund path. If Remita cannot settle, support refunds the ZEC or retries with a new reference.
- The receipt must show the RRR and the Remita reference, so the student can present it to the bursary.

## Validation checklist before writing code

1. Get a RemitaConnect account with Collections enabled and test keys.
2. Call `get-billers` and confirm UNN, UI, UNILAG, OAU, and UNIBEN are present with school-fee products.
3. Run `lookup` on one real UNN RRR and one real UI RRR. Record the response fields and status values.
4. Ask the bursary whether it accepts payment from a third-party payer when the RRR is correct.
5. Confirm the fee and settlement model. Decide whether the student pays the Remita fee.
6. Confirm amount limits. School fees are larger than airtime, so AML thresholds and proof of source may apply.

## Compliance

This is the same crypto-to-fiat movement as the utility payments, with larger tickets. zutility collects ZEC and settles naira to the biller. That is an off-ramp, not a custodial wallet.

If the current utility flow is compliant, school fees add amount thresholds and payer-name mismatches, not a new licence question. Do not pool funds. Settle per order. Keep the reference trail.

## Sources

- Remita API documentation. https://api.remita.net/
- Remita public Postman collection. https://api.remita.net/api/collections/24289864/2sAY4vi3YC?segregateAuth=true&versionTag=latest
- UNN portal. https://unnportal.unn.edu.ng/
- UNN portal bundle, routes and RRR labels. https://unnportal.unn.edu.ng/static/js/main.309b5d39.chunk.js
- UI acceptance fee notice. https://ui.edu.ng/news/20262027-admission-exercise-payment-acceptance-fee
- UI student portal. http://student-portal.ui.edu.ng/login
- UI bursary. https://bursary.ui.edu.ng/
- Existing Remita client. `zutility-be/src/integrations/remita/mod.rs`
- Provider routing. `zutility-be/src/integrations/provider_dispatcher.rs`
