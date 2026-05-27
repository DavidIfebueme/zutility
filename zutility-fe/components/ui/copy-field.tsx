"use client"

import * as React from "react"
import CheckedIcon from "@/components/icons/checked-icon"
import CopyIcon from "@/components/icons/copy-icon"
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
    } catch (err) {
      console.error("Failed to copy text: ", err)
    }
  }

  return (
    <div className={cn("flex flex-col gap-1.5", className)} {...props}>
      {label && <span className="text-xs text-text-secondary uppercase tracking-wider">{label}</span>}
      <div className="group relative flex items-center justify-between rounded-md border border-border-subtle bg-bg-elevated p-3 transition-colors hover:border-border-active">
        <span className="font-mono text-sm text-text-primary truncate mr-4 select-all">
          {value}
        </span>
        <button
          onClick={handleCopy}
          className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md bg-bg-surface text-text-secondary transition-colors hover:bg-border-subtle hover:text-text-primary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-zec"
          aria-label="Copy to clipboard"
          title="Copy"
        >
          {copied ? (
            <CheckedIcon size={16} className="text-accent-green" />
          ) : (
            <CopyIcon size={16} />
          )}
        </button>
        {copied && (
          <div className="absolute -top-8 right-0 rounded bg-bg-elevated px-2 py-1 text-xs text-accent-green shadow-md border border-border-subtle">
            Copied!
          </div>
        )}
      </div>
    </div>
  )
}
