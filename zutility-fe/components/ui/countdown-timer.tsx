"use client"

import * as React from "react"
import { cn } from "@/lib/utils"

interface CountdownTimerProps extends React.HTMLAttributes<HTMLDivElement> {
  expiresAt: string
  onExpire?: () => void
}

export function CountdownTimer({ expiresAt, onExpire, className, ...props }: CountdownTimerProps) {
  const [timeLeft, setTimeLeft] = React.useState<number>(0)
  const [isExpired, setIsExpired] = React.useState(false)

  React.useEffect(() => {
    const targetDate = new Date(expiresAt).getTime()

    const updateTimer = () => {
      const now = new Date().getTime()
      const difference = targetDate - now

      if (difference <= 0) {
        setTimeLeft(0)
        if (!isExpired) {
          setIsExpired(true)
          onExpire?.()
        }
      } else {
        setTimeLeft(difference)
      }
    }

    updateTimer()
    const interval = setInterval(updateTimer, 1000)

    return () => clearInterval(interval)
  }, [expiresAt, onExpire, isExpired])

  const minutes = Math.floor((timeLeft % (1000 * 60 * 60)) / (1000 * 60))
  const seconds = Math.floor((timeLeft % (1000 * 60)) / 1000)

  const isWarning = timeLeft > 0 && timeLeft <= 120000 // < 2 mins
  const isCritical = timeLeft > 0 && timeLeft <= 30000 // < 30 secs

  return (
    <div
      className={cn(
        "font-mono text-sm font-medium transition-colors",
        isCritical ? "text-accent-red animate-pulse" : isWarning ? "text-accent-zec" : "text-text-secondary",
        className
      )}
      {...props}
    >
      {isExpired ? "00:00" : `${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}`}
    </div>
  )
}
