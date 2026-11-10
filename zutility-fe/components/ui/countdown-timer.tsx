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
    const target = new Date(expiresAt).getTime()

    const update = () => {
      const difference = target - Date.now()
      if (difference <= 0) {
        setTimeLeft(0)
        setIsExpired((value) => {
          if (!value) onExpire?.()
          return true
        })
      } else {
        setTimeLeft(difference)
      }
    }

    update()
    const interval = setInterval(update, 1000)
    return () => clearInterval(interval)
  }, [expiresAt, onExpire])

  const minutes = Math.floor((timeLeft % (1000 * 60 * 60)) / (1000 * 60))
  const seconds = Math.floor((timeLeft % (1000 * 60)) / 1000)
  const isLow = timeLeft > 0 && timeLeft <= 120000

  return (
    <span
      className={cn("num text-[13px]", isLow ? "text-rust" : "text-ink", className)}
      {...props}
    >
      {isExpired ? "00:00" : `${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`}
    </span>
  )
}
