import * as React from "react"
import { cn } from "@/lib/utils"
import Image from "next/image"

interface TokenBadgeProps extends React.HTMLAttributes<HTMLDivElement> {
  token: 'ZEC' | 'COMING_SOON'
  label?: string
}

export function TokenBadge({ token, label, className, ...props }: TokenBadgeProps) {
  const isZec = token === 'ZEC'

  return (
    <div
      className={cn(
        "inline-flex items-center gap-2 rounded-full border px-3 py-1.5 text-sm font-medium transition-colors",
        isZec
          ? "border-accent-zec bg-accent-zec/10 text-text-primary shadow-[0_0_10px_rgba(244,183,40,0.2)]"
          : "border-border-subtle bg-bg-surface text-text-muted opacity-50",
        className
      )}
      {...props}
    >
      {isZec ? (
        <div className="flex h-5 w-5 items-center justify-center rounded-full bg-accent-zec text-bg-void font-bold text-[10px]">
          Z
        </div>
      ) : (
        <div className="h-5 w-5 rounded-full border border-dashed border-text-muted" />
      )}
      <span>{label || (isZec ? 'Zcash' : '[SOON]')}</span>
    </div>
  )
}
