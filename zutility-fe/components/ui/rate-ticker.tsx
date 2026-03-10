"use client"

import * as React from "react"
import { cn } from "@/lib/utils"
import { useRate } from "@/lib/hooks/useRate"
import { formatNGN } from "@/lib/utils"

interface RateTickerProps extends React.HTMLAttributes<HTMLDivElement> {
  showChange?: boolean
}

export function RateTicker({ showChange = false, className, ...props }: RateTickerProps) {
  const { rate, isLoading, isError, lastUpdated } = useRate()
  const [isFresh, setIsFresh] = React.useState(false)

  React.useEffect(() => {
    if (lastUpdated) {
      setIsFresh(Date.now() - new Date(lastUpdated).getTime() < 60000)
    }
  }, [lastUpdated])

  if (isLoading) {
    return (
      <div className={cn("flex items-center gap-2 text-sm text-text-muted", className)} {...props}>
        <div className="h-4 w-24 animate-pulse rounded bg-bg-surface" />
      </div>
    )
  }

  if (isError || !rate) {
    return (
      <div className={cn("text-sm text-accent-red", className)} {...props}>
        Rate unavailable
      </div>
    )
  }

  return (
    <div className={cn("flex items-center gap-3 text-sm font-medium", className)} {...props}>
      <div className="flex items-center gap-1.5">
        <span className="text-text-secondary">ZEC/NGN:</span>
        <span className="text-text-primary">{formatNGN(rate.zec_ngn)}</span>
      </div>
      
      {showChange && (
        <span className="text-accent-green flex items-center gap-0.5">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <polyline points="22 7 13.5 15.5 8.5 10.5 2 17"></polyline>
            <polyline points="16 7 22 7 22 13"></polyline>
          </svg>
          +2.4%
        </span>
      )}

      {isFresh && (
        <span className="relative flex h-2 w-2">
          <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-accent-green opacity-75"></span>
          <span className="relative inline-flex h-2 w-2 rounded-full bg-accent-green"></span>
        </span>
      )}
    </div>
  )
}
