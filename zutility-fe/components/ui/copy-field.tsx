"use client"

import * as React from "react"
import { Check, Copy } from "lucide-react"
import { cn } from "@/lib/utils"

interface CopyFieldProps extends React.HTMLAttributes<HTMLDivElement> {
  value: string
  label?: string
}

export function CopyField({ value, label, className, ...props }: CopyFieldProps) {
  const [copied, setCopied] = React.useState(false)

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(value)
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    } catch {
      setCopied(false)
    }
  }

  return (
    <div className={cn("flex flex-col gap-1.5", className)} {...props}>
      {label && <span className="label-xs text-muted">{label}</span>}
      <div className="flex items-center gap-2 rounded-md border border-line bg-paper px-3 py-2">
        <span className="num flex-1 truncate text-[13px] text-ink select-all">{value}</span>
        <button
          type="button"
          onClick={handleCopy}
          className="flex h-7 w-7 shrink-0 items-center justify-center rounded-md text-muted transition-colors hover:bg-paper-3 hover:text-ink"
          aria-label={copied ? "Copied" : "Copy to clipboard"}
        >
          {copied ? <Check className="h-3.5 w-3.5 text-green" /> : <Copy className="h-3.5 w-3.5" />}
        </button>
      </div>
      <span className="h-3 text-[11px] text-green" aria-live="polite">
        {copied ? "Copied" : ""}
      </span>
    </div>
  )
}
