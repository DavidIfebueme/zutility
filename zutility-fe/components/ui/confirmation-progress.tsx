"use client"

import * as React from "react"
import { cn } from "@/lib/utils"

interface ConfirmationProgressProps extends React.HTMLAttributes<HTMLDivElement> {
  current: number
  required: number
  addressType: 'shielded' | 'transparent'
}

export function ConfirmationProgress({
  current,
  required,
  addressType,
  className,
  ...props
}: ConfirmationProgressProps) {
  const segments = Math.max(required, 1)
  const resolved = Math.min(Math.max(current, 0), segments)

  return (
    <div className={cn("w-full", className)} {...props}>
      <div className="flex items-baseline justify-between">
        <span className="label-xs text-muted">Confirmations</span>
        <span className="num text-[13px] text-ink">
          {resolved} / {segments}
        </span>
      </div>
      <div className="mt-2 flex gap-1" role="progressbar" aria-valuenow={resolved} aria-valuemin={0} aria-valuemax={segments}>
        {Array.from({ length: segments }).map((_, index) => (
          <span
            key={index}
            className={cn(
              "h-1.5 flex-1 rounded-sm",
              index < resolved ? "bg-green" : "bg-paper-3"
            )}
          />
        ))}
      </div>
      <p className="mt-2 text-[12px] text-muted">
        {addressType === "shielded"
          ? "Shielded payment. One extra confirmation over a transparent send."
          : "Transparent payment."}
      </p>
    </div>
  )
}
