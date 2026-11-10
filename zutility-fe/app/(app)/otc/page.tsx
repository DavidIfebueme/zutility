"use client"

import Link from "next/link"
import { ArrowRight } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"

const STEPS = [
  {
    number: "01",
    title: "Name the amount",
    body: "Enter the local currency you want out, or the ZEC you want to sell.",
  },
  {
    number: "02",
    title: "Take the quote",
    body: "The quote shows the rate, the ZEC to send, and the payout. It holds while the order is open.",
  },
  {
    number: "03",
    title: "Send and wait",
    body: "Send ZEC to the order address. After the payment confirms, the payout goes to your account.",
  },
]

export default function OtcPage() {
  return (
    <div className="space-y-8">
      <header>
        <p className="label-xs text-muted">Off-ramp</p>
        <div className="mt-2 flex flex-wrap items-center gap-3">
          <h1 className="text-3xl text-ink">OTC off-ramp</h1>
          <Badge variant="coming-soon">Coming soon</Badge>
        </div>
        <p className="mt-2 max-w-xl text-[14px] leading-relaxed text-ink-2">
          Sell ZEC for local currency through a quote, instead of finding a buyer and agreeing on a
          price yourself.
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
              Not live yet. This page takes no deposit and holds no funds.
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
                <ArrowRight className="h-4 w-4" strokeWidth={1.8} />
              </Link>
            </Button>
          </div>
        </aside>
      </div>
    </div>
  )
}
