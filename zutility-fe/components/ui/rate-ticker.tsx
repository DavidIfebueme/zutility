"use client"

import * as React from "react"
import { cn } from "@/lib/utils"
import { useRate } from "@/lib/hooks/useRate"
import { useCurrency } from "@/lib/hooks/useCurrency"
import { formatCurrency, convertFromNGN, type FxRates } from "@/lib/currency"

interface RateTickerProps extends React.HTMLAttributes<HTMLDivElement> {}

export function RateTicker({ className, ...props }: RateTickerProps) {
  const { rate, isLoading, isError } = useRate()
  const currency = useCurrency()

  if (isLoading) {
    return (
      <div className={cn("flex items-center gap-2", className)} {...props}>
        <span className="h-3 w-20 animate-pulse rounded-sm bg-paper-3" />
      </div>
    )
  }

  if (isError || !rate) {
    return (
      <div className={cn("text-[13px] text-muted", className)} {...props}>
        Rate unavailable
      </div>
    )
  }

  const fxRates: FxRates = {
    usd_ngn: parseFloat(rate.usd_ngn) || 1,
    usd_kes: parseFloat(rate.usd_kes) || 0,
    usd_ghs: parseFloat(rate.usd_ghs) || 0,
    usd_zar: parseFloat(rate.usd_zar) || 0,
    usd_egp: parseFloat(rate.usd_egp) || 0,
  }

  const zecLocal = currency === 'NGN'
    ? parseFloat(rate.zec_ngn)
    : currency === 'USD'
      ? parseFloat(rate.zec_usd)
      : convertFromNGN(parseFloat(rate.zec_ngn), currency, fxRates)

  const label = currency === 'NGN' ? 'ZEC/NGN' : `ZEC/${currency}`

  return (
    <div className={cn("flex items-baseline gap-2 text-[13px]", className)} {...props}>
      <span className="label-xs text-muted">{label}</span>
      <span className="num text-ink">{formatCurrency(zecLocal, currency)}</span>
    </div>
  )
}
