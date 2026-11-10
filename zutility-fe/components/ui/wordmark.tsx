import Link from "next/link"
import { cn } from "@/lib/utils"

export function Wordmark({ className }: { className?: string }) {
  return (
    <span className={cn("inline-flex items-center gap-2", className)}>
      <span className="flex h-7 w-7 items-center justify-center rounded-md bg-ink font-display text-[15px] leading-none text-cream">
        z
      </span>
      <span className="font-display text-xl tracking-tight text-ink">utility</span>
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
