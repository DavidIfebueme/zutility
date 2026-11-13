"use client"

import * as React from "react"
import { Badge } from "@/components/ui/badge"
import { BrandTile } from "@/components/marketing/brand-tile"
import { cn } from "@/lib/utils"
import { useCurrency } from "@/lib/hooks/useCurrency"
import { useFxRates } from "@/lib/hooks/useFxRates"
import { formatLocalAmount } from "@/lib/currency"
import type { BrandIconId } from "@/lib/constants"

interface SampleOrder {
  id: string
  utility: string
  detail: string
  amountNgn: number
  zec: string
  address: string
  brand: BrandIconId
}

const SAMPLES: SampleOrder[] = [
  {
    id: "8F2C-41A9",
    utility: "MTN Airtime",
    detail: "0803 555 0142",
    amountNgn: 2000,
    zec: "0.00153524",
    address: "utest1q9x4v7k2m8p3n6r5t0w1y2z3a4b5c6d7e8f9g0h",
    brand: "mtn",
  },
  {
    id: "2B7D-90C4",
    utility: "Ikeja Electric",
    detail: "Meter 0412 3456 789",
    amountNgn: 10000,
    zec: "0.00767618",
    address: "utest1h7k2m8p3n6r5t0w1y2z3a4b5c6d7e8f9g0h1j2k3l4m5n6",
    brand: "ikeja-electric",
  },
  {
    id: "5A1E-73B8",
    utility: "School Fees",
    detail: "UNN · RRR 141523991664",
    amountNgn: 52000,
    zec: "0.03991616",
    address: "utest1m8p3n6r5t0w1y2z3a4b5c6d7e8f9g0h1j2k3l4m5n6p7q",
    brand: "unn",
  },
  {
    id: "C3F6-2D10",
    utility: "DStv Compact",
    detail: "Smartcard 7021 345 678",
    amountNgn: 19000,
    zec: "0.01458475",
    address: "utest1n6r5t0w1y2z3a4b5c6d7e8f9g0h1j2k3l4m5n6p7q8r9s",
    brand: "dstv",
  },
  {
    id: "9D4B-55E2",
    utility: "Airtel Data",
    detail: "10GB · 0802 555 0142",
    amountNgn: 5000,
    zec: "0.00383809",
    address: "utest1p3n6r5t0w1y2z3a4b5c6d7e8f9g0h1j2k3l4m5n6p7q8r9t",
    brand: "airtel",
  },
]

const STACK_DEPTH = 3
const FLIP_MS = 620

function ReceiptCard({ sample, stacked }: { sample: SampleOrder; stacked: boolean }) {
  const currency = useCurrency()
  const fxRates = useFxRates()

  return (
    <div
      className={cn(
        "rounded-lg border",
        stacked
          ? "border-line-strong bg-paper-3 shadow-[0_1px_0_0_var(--color-line)]"
          : "border-line bg-cream shadow-[0_24px_60px_-34px_rgba(20,19,15,0.6)]"
      )}
    >
      <div className="flex items-center justify-between border-b border-line px-5 py-3">
        <p className="label-xs text-muted">Order {sample.id}</p>
        <Badge variant="warning">Awaiting payment</Badge>
      </div>

      <div className="space-y-5 p-5">
        <div className="flex items-start gap-3">
          <BrandTile id={sample.brand} />
          <div className="min-w-0">
            <p className="truncate text-sm font-medium text-ink">{sample.utility}</p>
            <p className="num truncate text-[13px] text-muted">{sample.detail}</p>
          </div>
          <p className="num ml-auto shrink-0 text-lg text-ink">
            {fxRates ? formatLocalAmount(sample.amountNgn, currency, fxRates) : "—"}
          </p>
        </div>

        <div className="rounded-md border border-line bg-paper px-4 py-3">
          <p className="label-xs text-muted">Send exactly</p>
          <p className="num mt-1 text-2xl text-ink">{sample.zec} ZEC</p>
          <p className="num mt-1 truncate text-[12px] text-muted">{sample.address}</p>
        </div>

        <dl className="space-y-2 text-[13px]">
          <div className="flex items-center justify-between">
            <dt className="text-muted">Network fee</dt>
            <dd className="text-ink">Paid by sender</dd>
          </div>
          <div className="flex items-center justify-between">
            <dt className="text-muted">Settlement</dt>
            <dd className="text-ink">About a minute</dd>
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
  )
}

export function SampleReceiptStack() {
  const [active, setActive] = React.useState(0)
  const [flipping, setFlipping] = React.useState(false)
  const [paused, setPaused] = React.useState(false)

  React.useEffect(() => {
    if (paused) return
    if (window.matchMedia?.("(prefers-reduced-motion: reduce)").matches) return

    let flipTimer: number | undefined
    const timer = window.setInterval(() => {
      setFlipping(true)
      flipTimer = window.setTimeout(() => {
        setActive((value) => (value + 1) % SAMPLES.length)
        setFlipping(false)
      }, FLIP_MS)
    }, 4200)

    return () => {
      window.clearInterval(timer)
      if (flipTimer) window.clearTimeout(flipTimer)
    }
  }, [paused])

  return (
    <div className="w-full max-w-md">
      <div
        className="relative h-[474px]"
        style={{ perspective: "1600px" }}
        onMouseEnter={() => setPaused(true)}
        onMouseLeave={() => setPaused(false)}
      >
        {SAMPLES.map((sample, index) => {
          const offset = (index - active + SAMPLES.length) % SAMPLES.length
          const depth = Math.min(offset, STACK_DEPTH)
          const isFront = offset === 0
          const isNext = offset === 1

          const restTransform = `translateY(${depth * 22}px) scale(${1 - depth * 0.045})`
          const transform = isFront && flipping
            ? "rotateY(-108deg)"
            : isNext && flipping
              ? "translateY(0px) scale(1)"
              : restTransform

          return (
            <div
              key={sample.id}
              aria-hidden={!isFront}
              className="absolute inset-x-0 top-0 will-change-transform"
              style={{
                zIndex: SAMPLES.length - offset,
                opacity: offset > STACK_DEPTH ? 0 : 1,
                transform,
                transformOrigin: isFront ? "left center" : "center top",
                backfaceVisibility: "hidden",
                transition: `transform ${FLIP_MS}ms cubic-bezier(0.22, 1, 0.36, 1)`,
                pointerEvents: isFront ? "auto" : "none",
              }}
            >
              <ReceiptCard sample={sample} stacked={!isFront} />
            </div>
          )
        })}
      </div>

      <div className="mt-3 flex items-center justify-end gap-4 px-1">
        <div className="flex items-center gap-1.5">
          {SAMPLES.map((sample, index) => (
            <button
              key={sample.id}
              type="button"
              onClick={() => setActive(index)}
              aria-label={`Show ${sample.utility}`}
              aria-current={index === active}
              className={cn(
                "h-1.5 rounded-full transition-all duration-300",
                index === active ? "w-5 bg-gold-deep" : "w-1.5 bg-line-strong hover:bg-muted"
              )}
            />
          ))}
        </div>
      </div>
    </div>
  )
}
