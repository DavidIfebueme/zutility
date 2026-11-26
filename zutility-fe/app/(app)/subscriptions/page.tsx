"use client"

import Link from "next/link"
import { ArrowRight } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"

const STEPS = [
  {
    number: "01",
    title: "Choose a plan",
    body: "Pick a service and a cycle, such as monthly data, a TV package, or electricity.",
  },
  {
    number: "02",
    title: "Approve once",
    body: "Your wallet signs the terms once. A charge can never exceed them, and you can stop it at any time.",
  },
  {
    number: "03",
    title: "Every cycle delivers",
    body: "The cycle is paid and delivered on schedule, and the receipt lands in your history.",
  },
]

export default function SubscriptionsPage() {
  return (
    <div className="space-y-8">
      <header>
        <p className="label-xs text-muted">Recurring</p>
        <div className="mt-2 flex flex-wrap items-center gap-3">
          <h1 className="text-3xl text-ink">Subscriptions</h1>
          <Badge variant="coming-soon">Coming soon</Badge>
        </div>
        <p className="mt-2 max-w-xl text-[14px] leading-relaxed text-ink-2">
          Subscribe to services that support native ZEC subscriptions. Approve the terms once in
          your wallet, each cycle is paid automatically, and you can stop it whenever you want.
        </p>
      </header>

      <div className="grid gap-6 lg:grid-cols-[1fr_236px]">
        <section className="rounded-lg border border-line bg-cream">
          <div className="border-b border-line px-5 py-3">
            <p className="label-xs text-muted">What it will do</p>
          </div>

          <ol>
            {STEPS.map((step, index) => (
              <li
                key={step.number}
                className={
                  index === 0
                    ? "px-5 py-5"
                    : "border-t border-line px-5 py-5"
                }
              >
                <p className="num text-[11px] text-gold-deep">{step.number}</p>
                <h2 className="mt-1.5 text-base text-ink">{step.title}</h2>
                <p className="mt-1 max-w-md text-[13px] leading-relaxed text-ink-2">{step.body}</p>
              </li>
            ))}
          </ol>

          <div className="border-t border-line px-5 py-3">
            <p className="text-[12px] leading-relaxed text-muted">
              Not live yet. Nothing is charged and no mandate can be created. Native ZEC
              subscriptions use ELSA, which needs two Zcash network upgrades before it can run on
              mainnet.
            </p>
          </div>
        </section>

        <aside>
          <div className="rounded-lg border border-line bg-paper-2 p-4">
            <p className="label-xs text-muted">Available now</p>
            <p className="mt-2 text-[14px] text-ink">Pay a bill</p>
            <p className="mt-1 text-[13px] leading-relaxed text-ink-2">
              Airtime, data, TV, prepaid electricity, exam PINs, and school fees. Pay with shielded
              ZEC.
            </p>
            <Button asChild className="mt-4 w-full" size="sm">
              <Link href="/pay">
                Pay a bill
                <ArrowRight className="h-4 w-4 transition-transform duration-200 group-hover/btn:translate-x-0.5" strokeWidth={1.8} />
              </Link>
            </Button>
          </div>
        </aside>
      </div>
    </div>
  )
}
