"use client"

import Link from "next/link"
import { ArrowRight } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"

const STEPS = [
  {
    number: "01",
    title: "Browse the offers",
    body: "Each offer lists the price and the limits that trader has set.",
  },
  {
    number: "02",
    title: "Escrow holds the ZEC",
    body: "The seller's ZEC stays in escrow for the trade. Neither side can move it while the order is open.",
  },
  {
    number: "03",
    title: "Settle the trade",
    body: "Once the local currency payment clears, the escrow releases the ZEC to the buyer.",
  },
]

export default function P2pPage() {
  return (
    <div className="space-y-8">
      <header>
        <p className="label-xs text-muted">Marketplace</p>
        <div className="mt-2 flex flex-wrap items-center gap-3">
          <h1 className="text-3xl text-ink">P2P market</h1>
          <Badge variant="coming-soon">Coming soon</Badge>
        </div>
        <p className="mt-2 max-w-xl text-[14px] leading-relaxed text-ink-2">
          Buy and sell ZEC with other people at a price you agree on, rather than a rate set for
          you.
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
              Not live yet. No offers, escrow, or deposits are open.
            </p>
          </div>
        </section>

        <aside>
          <div className="rounded-lg border border-line bg-paper-2 p-4">
            <p className="label-xs text-muted">Available now</p>
            <p className="mt-2 text-[14px] text-ink">Pay a bill</p>
            <p className="mt-1 text-[13px] leading-relaxed text-ink-2">
              Airtime, data, TV, prepaid electricity, exam PINs, and school fees. Pay with shielded
              or transparent ZEC.
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
