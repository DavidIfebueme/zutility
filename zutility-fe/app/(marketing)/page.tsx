import Link from "next/link"
import { ArrowRight, Check, Clock, Lock, Wallet } from "lucide-react"
import { SiteHeader } from "@/components/marketing/site-header"
import { SiteFooter } from "@/components/marketing/site-footer"
import { BrandTile } from "@/components/marketing/brand-tile"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { RateTicker } from "@/components/ui/rate-ticker"
import { Reveal } from "@/components/ui/reveal"
import { UTILITIES } from "@/lib/constants"

const HERO_FACTS = [
  { icon: Clock, label: "Settlement", value: "About 4 minutes after broadcast" },
  { icon: Wallet, label: "Payment", value: "Shielded or transparent ZEC" },
  { icon: Lock, label: "Identity", value: "No documents, no account approval" },
]

const STEPS = [
  {
    number: "01",
    title: "Pick the bill",
    body: "Choose a service and enter the meter, smartcard, or phone number. We check the account with the provider before you send anything.",
  },
  {
    number: "02",
    title: "Send the exact amount",
    body: "The order gets one Zcash deposit address and one ZEC amount, quoted at the current rate. Pay from any Zcash wallet, shielded or transparent.",
  },
  {
    number: "03",
    title: "Collect the receipt",
    body: "We watch that address, wait for confirmations, then dispatch the top-up, token, or subscription. The receipt stays on the order page.",
  },
]

const FAQ = [
  {
    q: "Do I need to create an account?",
    a: "No. You can create an order and pay from any Zcash wallet. An account only keeps your order history in one place.",
  },
  {
    q: "How long does delivery take?",
    a: "The network confirmation window is about four minutes at the current block target, then the provider dispatch is usually instant. Some meters and exam PINs take a little longer.",
  },
  {
    q: "What happens if I send the wrong amount?",
    a: "Send the exact amount shown on the order. If you underpay, the order stays open until it expires and support can help you recover the funds. If you overpay, the surplus is returned to the sending address when possible.",
  },
  {
    q: "Can I pay from a shielded address?",
    a: "Yes. The order accepts shielded or transparent ZEC. A shielded payment needs one more confirmation than a transparent one.",
  },
  {
    q: "Which utilities are covered?",
    a: "Airtime and data on MTN, Airtel, Glo, and 9mobile. DSTV, GOtv, Startimes, and Showmax. Prepaid electricity across twelve distribution companies. WAEC, JAMB, and school fees.",
  },
]

const FEATURED_NAMES = [
  "MTN",
  "Airtel",
  "Glo",
  "9mobile",
  "DSTV",
  "GOtv",
  "Startimes",
  "Showmax",
  "Ikeja Electric",
  "Eko Electric",
  "Abuja Electric",
  "WAEC",
  "JAMB",
]

function SampleReceipt() {
  return (
    <div className="w-full max-w-md">
      <div className="rounded-lg border border-line bg-cream transition-[transform,border-color] duration-300 ease-out hover:-translate-y-1 hover:border-line-strong">
        <div className="flex items-center justify-between border-b border-line px-5 py-3">
          <p className="label-xs text-muted">Order 8F2C-41A9</p>
          <Badge variant="warning">Awaiting payment</Badge>
        </div>
        <div className="space-y-5 p-5">
          <div className="flex items-start gap-3">
            <BrandTile id="mtn" />
            <div className="min-w-0">
              <p className="text-sm font-medium text-ink">MTN Airtime</p>
              <p className="num text-[13px] text-muted">0803 555 0142</p>
            </div>
            <p className="num ml-auto text-lg text-ink">₦2,000.00</p>
          </div>

          <div className="rounded-md border border-line bg-paper px-4 py-3">
            <p className="label-xs text-muted">Send exactly</p>
            <p className="num mt-1 text-2xl text-ink">0.08430000 ZEC</p>
            <p className="num mt-1 truncate text-[12px] text-muted">
              u1q9x4v7k2m8p3n6r5t0w1y2z3a4b5c6d7e8f9g0h
            </p>
          </div>

          <dl className="space-y-2 text-[13px]">
            <div className="flex items-center justify-between">
              <dt className="text-muted">Network fee</dt>
              <dd className="text-ink">Paid by sender</dd>
            </div>
            <div className="flex items-center justify-between">
              <dt className="text-muted">Confirmations</dt>
              <dd className="num text-ink">0 of 9</dd>
            </div>
            <div className="flex items-center justify-between">
              <dt className="text-muted">Expires in</dt>
              <dd className="num text-ink">29:59</dd>
            </div>
          </dl>
        </div>
        <div className="border-t border-line px-5 py-3">
          <p className="text-[12px] leading-relaxed text-muted">
            We credit the order when a payment reaches this address. Nothing is credited from a
            wallet scan.
          </p>
        </div>
      </div>
      <p className="mt-3 px-1 text-[12px] text-muted">
        Sample receipt. A live order shows the current rate and its own address.
      </p>
    </div>
  )
}

export default function HomePage() {
  const counts = UTILITIES.reduce<Record<string, number>>((acc, utility) => {
    acc[utility.type] = (acc[utility.type] ?? 0) + 1
    return acc
  }, {})

  return (
    <div className="min-h-screen bg-paper">
      <SiteHeader />

      <main>
        <section className="paper-grid border-b border-line">
          <div className="mx-auto grid max-w-6xl items-center gap-12 px-5 py-16 lg:grid-cols-[1.02fr_0.98fr] lg:gap-16 lg:py-24">
            <div className="reveal-in">
              <p className="label-xs text-gold-deep">Utility payments on Zcash</p>
              <h1 className="mt-4 max-w-xl text-4xl leading-[1.05] text-ink sm:text-5xl lg:text-[56px]">
                Pay electricity, airtime, and TV with ZEC.
              </h1>
              <p className="mt-5 max-w-lg text-[15px] leading-relaxed text-ink-2">
                Send ZEC from any wallet and get a prepaid meter token, a phone top-up, or a TV
                renewal. One address, one amount, one receipt. No identity documents.
              </p>

              <div className="mt-7 flex flex-wrap items-center gap-3">
                <Button asChild size="lg">
                  <Link href="/pay">
                    Pay a bill
                    <ArrowRight className="h-4 w-4 transition-transform duration-200 group-hover/btn:translate-x-0.5" />
                  </Link>
                </Button>
                <Button asChild variant="secondary" size="lg">
                  <Link href="/how-it-works">See how it works</Link>
                </Button>
              </div>

              <dl className="mt-10 grid gap-5 border-t border-line pt-6 sm:grid-cols-3">
                {HERO_FACTS.map((fact) => (
                  <div key={fact.label}>
                    <dt className="label-xs flex items-center gap-1.5 text-muted">
                      <fact.icon className="h-3.5 w-3.5" strokeWidth={1.6} />
                      {fact.label}
                    </dt>
                    <dd className="mt-1.5 text-[13px] leading-snug text-ink">{fact.value}</dd>
                  </div>
                ))}
              </dl>
            </div>

            <div className="flex justify-center lg:justify-end">
              <Reveal delay={100} className="w-full max-w-md">
                <SampleReceipt />
              </Reveal>
            </div>
          </div>
        </section>

        <section className="border-b border-line bg-cream">
          <div className="mx-auto flex max-w-6xl flex-wrap items-center gap-x-6 gap-y-3 px-5 py-7">
            <p className="label-xs text-muted">Pay for</p>
            <ul className="flex flex-wrap items-center gap-x-5 gap-y-2">
              {FEATURED_NAMES.map((name) => (
                <li key={name} className="text-[13px] text-ink-2">
                  {name}
                </li>
              ))}
              <li className="text-[13px] text-muted">and more</li>
            </ul>
          </div>
        </section>

        <section className="border-b border-line">
          <div className="mx-auto max-w-6xl px-5 py-16 lg:py-20">
            <div className="flex flex-col justify-between gap-4 sm:flex-row sm:items-end">
              <div>
                <p className="label-xs text-muted">Three steps</p>
                <h2 className="mt-3 text-3xl text-ink sm:text-4xl">From wallet to meter in one sitting.</h2>
              </div>
              <Link
                href="/how-it-works"
                className="u-link inline-flex items-center gap-1.5 text-[13px] text-gold-deep"
              >
                Read the full flow
                <ArrowRight className="h-3.5 w-3.5 transition-transform duration-200 group-hover/btn:translate-x-0.5" />
              </Link>
            </div>

            <div className="mt-10 grid gap-0 border-t border-line md:grid-cols-3">
              {STEPS.map((step, index) => (
                <Reveal
                  key={step.number}
                  delay={index * 80}
                  className={
                    index === 0
                      ? "py-7 md:pr-8"
                      : "border-t border-line py-7 md:border-l md:border-t-0 md:px-8"
                  }
                >
                  <p className="num text-[12px] text-gold-deep">{step.number}</p>
                  <h3 className="mt-3 text-xl text-ink">{step.title}</h3>
                  <p className="mt-2 text-[14px] leading-relaxed text-ink-2">{step.body}</p>
                </Reveal>
              ))}
            </div>
          </div>
        </section>

        <section id="rates" className="border-b border-line bg-paper-2">
          <div className="mx-auto grid max-w-6xl gap-12 px-5 py-16 lg:grid-cols-2 lg:py-20">
            <Reveal>
              <p className="label-xs text-muted">Rates and coverage</p>
              <h2 className="mt-3 text-3xl text-ink sm:text-4xl">
                The rate you see is the rate the order quotes.
              </h2>
              <p className="mt-4 max-w-md text-[14px] leading-relaxed text-ink-2">
                Rates refresh every minute from the market feed. When you create an order, the ZEC
                amount is locked for the life of that order, so a moving market does not change
                what you owe.
              </p>

              <div className="mt-8 inline-flex flex-col gap-1 rounded-lg border border-line bg-cream px-5 py-4">
                <span className="label-xs text-muted">Current rate</span>
                <RateTicker className="text-[22px]" />
                <span className="text-[12px] text-muted">
                  Refreshed every minute. Orders keep their quoted amount.
                </span>
              </div>

              <ul className="mt-8 space-y-2.5">
                {[
                  "Prepaid tokens from twelve distribution companies",
                  "Airtime and data on the four major networks",
                  "TV renewals and exam PINs",
                  "Payment accepted from shielded or transparent addresses",
                ].map((line) => (
                  <li key={line} className="flex items-start gap-2 text-[14px] text-ink-2">
                    <Check className="mt-0.5 h-4 w-4 shrink-0 text-green" strokeWidth={2} />
                    {line}
                  </li>
                ))}
              </ul>
            </Reveal>

            <Reveal delay={90} className="rounded-lg border border-line bg-cream">
              <div className="border-b border-line px-5 py-4">
                <p className="label-xs text-muted">Live coverage</p>
                <p className="mt-1 text-[13px] text-ink-2">
                  {UTILITIES.length} services enabled on this deployment.
                </p>
              </div>
              <dl>
                {[
                  { label: "Airtime", type: "airtime" },
                  { label: "Data", type: "data" },
                  { label: "TV", type: "tv" },
                  { label: "Electricity", type: "electricity" },
                  { label: "Education", type: "education" },
                  { label: "School fees", type: "school" },
                ].map((row) => (
                  <div
                    key={row.type}
                    className="flex items-center justify-between border-b border-line px-5 py-3 last:border-b-0 transition-colors hover:bg-paper-2/60"
                  >
                    <dt className="text-[14px] text-ink">{row.label}</dt>
                    <dd className="num text-[13px] text-muted">
                      {String(counts[row.type] ?? 0).padStart(2, "0")}
                    </dd>
                  </div>
                ))}
              </dl>
            </Reveal>
          </div>
        </section>

        <section className="border-b border-line">
          <div className="mx-auto grid max-w-6xl gap-10 px-5 py-16 lg:grid-cols-[0.9fr_1.1fr] lg:py-20">
            <div>
              <p className="label-xs text-muted">Privacy</p>
              <h2 className="mt-3 text-3xl text-ink sm:text-4xl">
                What we see, and what we never ask for.
              </h2>
              <p className="mt-4 max-w-md text-[14px] leading-relaxed text-ink-2">
                A utility payment needs a meter number. It does not need your name, your ID, or a
                photograph of your face. We keep the order data required to deliver the service and
                nothing else.
              </p>
            </div>

            <Reveal delay={90} className="grid gap-6 sm:grid-cols-2">
              <div className="rounded-lg border border-line bg-cream p-5 transition-colors duration-200 hover:border-line-strong">
                <p className="label-xs text-muted">We hold</p>
                <ul className="mt-4 space-y-2.5 text-[14px] text-ink-2">
                  <li>The order amount and the service reference</li>
                  <li>The deposit address for that order</li>
                  <li>The transaction that pays it</li>
                </ul>
              </div>
              <div className="rounded-lg border border-line bg-cream p-5 transition-colors duration-200 hover:border-line-strong">
                <p className="label-xs text-muted">We do not hold</p>
                <ul className="mt-4 space-y-2.5 text-[14px] text-ink-2">
                  <li>Your wallet balance or seed</li>
                  <li>Your other addresses</li>
                  <li>Your identity documents</li>
                </ul>
              </div>
            </Reveal>
          </div>
        </section>

        <section>
          <div className="mx-auto max-w-3xl px-5 py-16 lg:py-20">
            <p className="label-xs text-muted">Questions</p>
            <h2 className="mt-3 text-3xl text-ink sm:text-4xl">The short answers.</h2>
            <Reveal className="mt-8 border-t border-line">
              {FAQ.map((item) => (
                <details key={item.q} className="group border-b border-line py-4">
                  <summary className="flex cursor-pointer list-none items-center justify-between gap-4 text-[15px] text-ink transition-colors duration-150 hover:text-gold-deep">
                    {item.q}
                    <span className="text-muted transition-transform duration-200 group-open:rotate-45">+</span>
                  </summary>
                  <p className="mt-3 max-w-2xl text-[14px] leading-relaxed text-ink-2">{item.a}</p>
                </details>
              ))}
            </Reveal>
            <div className="mt-10 flex flex-wrap items-center gap-3">
              <Button asChild size="lg">
                <Link href="/pay">Start an order</Link>
              </Button>
              <Button asChild variant="secondary" size="lg">
                <Link href="/support">Talk to support</Link>
              </Button>
            </div>
          </div>
        </section>
      </main>

      <SiteFooter />
    </div>
  )
}
