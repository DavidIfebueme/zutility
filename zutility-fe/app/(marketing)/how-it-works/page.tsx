import Link from "next/link"
import { ArrowRight, Clock, Lock, ShieldCheck } from "lucide-react"
import { SiteHeader } from "@/components/marketing/site-header"
import { SiteFooter } from "@/components/marketing/site-footer"
import { Button } from "@/components/ui/button"

const STEPS = [
  {
    id: "step-1",
    number: "01",
    title: "Pick the bill",
    body: "Choose airtime, data, a TV renewal, prepaid electricity, an exam PIN, or school fees. Enter the phone number, meter number, or smartcard number. We check the account with the provider before you send anything.",
  },
  {
    id: "step-2",
    number: "02",
    title: "Send the exact amount",
    body: "The order gets one Zcash deposit address and one ZEC amount, quoted at the current rate. Pay from any Zcash wallet. Shielded and transparent both work. Copy the address or scan the code on the order page.",
  },
  {
    id: "step-3",
    number: "03",
    title: "Wait for confirmations",
    body: "We watch that address for a payment that matches the order. A transparent payment needs 9 confirmations. A shielded payment needs 10. Blocks arrive about every 25 seconds, so the wait is roughly four minutes.",
  },
  {
    id: "step-4",
    number: "04",
    title: "Collect the delivery",
    body: "Once the confirmations land we dispatch the top-up, token, or subscription. The receipt stays on the order page. If you made an account, it also stays in your order history.",
  },
]

const INDEX = [
  { href: "#step-1", label: "Pick the bill" },
  { href: "#step-2", label: "Send the exact amount" },
  { href: "#step-3", label: "Wait for confirmations" },
  { href: "#step-4", label: "Collect the delivery" },
  { href: "#timing", label: "Timing" },
  { href: "#edge-cases", label: "When something goes wrong" },
  { href: "#privacy", label: "Privacy" },
]

const EDGE_CASES = [
  {
    term: "Underpayment",
    body: "The order stays open until its countdown ends, and the payment is short of the quoted amount. We do not dispatch a partial top-up. Support can trace what arrived and tell you whether a top-up of the balance or a return makes sense.",
  },
  {
    term: "Overpayment",
    body: "The order is still dispatched for the amount billed. The surplus sits with the order, and support can return it to the sending address when the wallet allows it.",
  },
  {
    term: "Expiry",
    body: "Each order shows a countdown on the order page. A payment that lands after expiry is not credited automatically. Support checks it and either re-quotes the order or returns the funds.",
  },
  {
    term: "Rejected account",
    body: "A meter, smartcard, or phone number can fail the provider check. Nothing is sent and nothing is charged. Correct the number and start the order again.",
  },
  {
    term: "Support",
    body: "Send the order ID and the deposit address. We trace the payment against that address, not a wallet scan, and work the case with the provider when a delivery fails.",
  },
]

export default function HowItWorksPage() {
  return (
    <div className="min-h-screen bg-paper">
      <SiteHeader />

      <main>
        <section className="paper-grid border-b border-line">
          <div className="mx-auto max-w-6xl px-5 py-16 lg:py-20">
            <p className="label-xs text-gold-deep">How it works</p>
            <h1 className="mt-4 max-w-3xl text-4xl leading-[1.08] text-ink sm:text-5xl">
              From a bill to a receipt in about four minutes.
            </h1>
            <p className="mt-5 max-w-2xl text-[15px] leading-relaxed text-ink-2">
              One order holds one deposit address and one ZEC amount. We watch that address, wait
              for the network to confirm the payment, then deliver the utility. No account approval
              and no identity documents.
            </p>

            <dl className="mt-10 grid gap-6 border-t border-line pt-6 sm:grid-cols-3">
              <div>
                <dt className="label-xs flex items-center gap-1.5 text-muted">
                  <Clock className="h-4 w-4" strokeWidth={1.6} />
                  Typical wait
                </dt>
                <dd className="mt-1.5 text-[13px] text-ink">About four minutes</dd>
              </div>
              <div>
                <dt className="label-xs flex items-center gap-1.5 text-muted">
                  <ShieldCheck className="h-4 w-4" strokeWidth={1.6} />
                  Payment
                </dt>
                <dd className="mt-1.5 text-[13px] text-ink">Shielded or transparent ZEC</dd>
              </div>
              <div>
                <dt className="label-xs flex items-center gap-1.5 text-muted">
                  <Lock className="h-4 w-4" strokeWidth={1.6} />
                  Identity
                </dt>
                <dd className="mt-1.5 text-[13px] text-ink">No documents at any point</dd>
              </div>
            </dl>
          </div>
        </section>

        <section className="border-b border-line">
          <div className="mx-auto grid max-w-6xl gap-12 px-5 py-14 lg:grid-cols-[220px_1fr] lg:py-20">
            <nav aria-label="On this page" className="hidden lg:block">
              <div className="sticky top-24">
                <p className="label-xs text-muted">On this page</p>
                <ul className="mt-4 space-y-2.5 border-t border-line pt-4">
                  {INDEX.map((item) => (
                    <li key={item.href}>
                      <a
                        href={item.href}
                        className="text-[13px] text-muted transition-colors hover:text-ink"
                      >
                        {item.label}
                      </a>
                    </li>
                  ))}
                </ul>
              </div>
            </nav>

            <div>
              <div className="border-t border-line">
                {STEPS.map((step) => (
                  <article
                    key={step.id}
                    id={step.id}
                    className="grid scroll-mt-24 gap-3 border-b border-line py-8 sm:grid-cols-[72px_1fr] sm:gap-6"
                  >
                    <p className="num text-[13px] text-gold-deep">{step.number}</p>
                    <div>
                      <h2 className="text-2xl text-ink">{step.title}</h2>
                      <p className="mt-2 max-w-2xl text-[14px] leading-relaxed text-ink-2">
                        {step.body}
                      </p>
                    </div>
                  </article>
                ))}
              </div>

              <div id="timing" className="scroll-mt-24 pt-12">
                <p className="label-xs text-muted">Timing</p>
                <h2 className="mt-3 text-2xl text-ink">What the clock covers.</h2>
                <p className="mt-2 max-w-2xl text-[14px] leading-relaxed text-ink-2">
                  The Zcash network targets a block every 25 seconds. An order waits for a fixed
                  number of blocks before it counts as paid.
                </p>

                <table className="mt-6 w-full border-t border-line text-left">
                  <thead>
                    <tr className="border-b border-line">
                      <th scope="col" className="py-3 pr-4 label-xs text-muted">
                        Stage
                      </th>
                      <th scope="col" className="py-3 label-xs text-muted">
                        Value
                      </th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr className="border-b border-line">
                      <td className="py-3 pr-4 text-[14px] text-ink">Block target</td>
                      <td className="num py-3 text-[13px] text-ink-2">about 25 seconds</td>
                    </tr>
                    <tr className="border-b border-line">
                      <td className="py-3 pr-4 text-[14px] text-ink">Transparent payment</td>
                      <td className="num py-3 text-[13px] text-ink-2">9 confirmations</td>
                    </tr>
                    <tr className="border-b border-line">
                      <td className="py-3 pr-4 text-[14px] text-ink">Shielded payment</td>
                      <td className="num py-3 text-[13px] text-ink-2">10 confirmations</td>
                    </tr>
                    <tr className="border-b border-line">
                      <td className="py-3 pr-4 text-[14px] text-ink">Total wait after broadcast</td>
                      <td className="num py-3 text-[13px] text-ink-2">about four minutes</td>
                    </tr>
                    <tr className="border-b border-line">
                      <td className="py-3 pr-4 text-[14px] text-ink">Delivery after confirmations</td>
                      <td className="py-3 text-[13px] text-ink-2">
                        Usually immediate. Some meters and exam PINs take longer.
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>

              <div id="edge-cases" className="scroll-mt-24 pt-12">
                <p className="label-xs text-muted">When something goes wrong</p>
                <h2 className="mt-3 text-2xl text-ink">The amount, the clock, the account.</h2>
                <p className="mt-2 max-w-2xl text-[14px] leading-relaxed text-ink-2">
                  Most failed orders come from one of these. None of them are silent. The order
                  page shows the current state, and support reads the same record.
                </p>

                <dl className="mt-6 border-t border-line">
                  {EDGE_CASES.map((item) => (
                    <div
                      key={item.term}
                      className="grid gap-2 border-b border-line py-5 sm:grid-cols-[180px_1fr] sm:gap-6"
                    >
                      <dt className="text-[14px] font-medium text-ink">{item.term}</dt>
                      <dd className="max-w-2xl text-[14px] leading-relaxed text-ink-2">
                        {item.body}
                      </dd>
                    </div>
                  ))}
                </dl>
              </div>

              <div id="privacy" className="scroll-mt-24 pt-12">
                <p className="label-xs text-muted">Privacy</p>
                <h2 className="mt-3 text-2xl text-ink">What the order holds.</h2>
                <div className="mt-6 grid gap-8 border-t border-line pt-6 sm:grid-cols-2">
                  <div>
                    <p className="text-[14px] font-medium text-ink">On the order</p>
                    <ul className="mt-3 space-y-2 text-[14px] text-ink-2">
                      <li>The amount and the service reference</li>
                      <li>The deposit address for that order</li>
                      <li>The transaction that pays it</li>
                    </ul>
                  </div>
                  <div>
                    <p className="text-[14px] font-medium text-ink">Never asked for</p>
                    <ul className="mt-3 space-y-2 text-[14px] text-ink-2">
                      <li>Your wallet seed or balance</li>
                      <li>Your other addresses</li>
                      <li>Your identity documents</li>
                    </ul>
                  </div>
                </div>
              </div>

              <div className="mt-12 flex flex-wrap items-center gap-3 border-t border-line pt-8">
                <Button asChild size="lg">
                  <Link href="/pay">
                    Start an order
                    <ArrowRight className="h-4 w-4 transition-transform duration-200 group-hover/btn:translate-x-0.5" strokeWidth={1.8} />
                  </Link>
                </Button>
                <Button asChild variant="secondary" size="lg">
                  <Link href="/support">Talk to support</Link>
                </Button>
              </div>
            </div>
          </div>
        </section>
      </main>

      <SiteFooter />
    </div>
  )
}
