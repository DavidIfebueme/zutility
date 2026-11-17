"use client"

import * as React from "react"
import { cn } from "@/lib/utils"
import { useRate } from "@/lib/hooks/useRate"
import { useCurrency } from "@/lib/hooks/useCurrency"
import { convertFromNGN, formatCurrency, fxRatesFromRate, type CurrencyCode } from "@/lib/currency"

interface RateTickerProps extends React.HTMLAttributes<HTMLDivElement> {
  initialCurrency?: CurrencyCode | null
}

export function RateTicker({ className, initialCurrency, ...props }: RateTickerProps) {
  const { rate, isLoading, isError } = useRate()
  const currency = useCurrency(initialCurrency)
  const label = `ZEC/${currency}`

  if (isLoading) {
    return (
      <div className={cn("flex items-center gap-2", className)} {...props}>
        <span className="label-xs text-muted">{label}</span>
        <span className="h-3 w-20 animate-pulse rounded-sm bg-paper-3" />
      </div>
    )
  }

  if (isError || !rate) {
    return (
      <div className={cn("flex items-center gap-2 text-[13px]", className)} {...props}>
        <span className="label-xs text-muted">{label}</span>
        <span className="text-muted">Rate unavailable</span>
      </div>
    )
  }

  const zecLocal = currency === 'NGN'
    ? parseFloat(rate.zec_ngn)
    : currency === 'USD'
      ? parseFloat(rate.zec_usd)
      : convertFromNGN(parseFloat(rate.zec_ngn), currency, fxRatesFromRate(rate))

  return (
    <div className={cn("flex items-baseline gap-2 text-[13px]", className)} {...props}>
      <span className="label-xs text-muted">{label}</span>
      <span className="num text-ink">{formatCurrency(zecLocal, currency)}</span>
    </div>
  )
}
