import Link from "next/link"
import { cn } from "@/lib/utils"

export function Wordmark({ className }: { className?: string }) {
  return (
    <span
      className={cn(
        "inline-flex items-baseline font-display text-xl leading-none tracking-[-0.055em] text-ink",
        className
      )}
    >
      <span className="text-gold">z</span>utility
    </span>
  )
}

export function WordmarkLink({ className }: { className?: string }) {
  return (
    <Link href="/" className={cn("inline-flex items-center", className)} aria-label="zutility home">
      <Wordmark />
    </Link>
  )
}
