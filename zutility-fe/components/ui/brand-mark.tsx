import Image from "next/image"
import type { BrandIconId } from "@/lib/constants"
import { BRAND_LOGOS, BRAND_MONOGRAMS } from "@/lib/brand-logo"

interface BrandMarkProps {
  id: BrandIconId
  size?: number
  className?: string
}

export function BrandMark({ id, size = 24, className }: BrandMarkProps) {
  const src = BRAND_LOGOS[id]

  if (!src) {
    return (
      <span
        className={`num font-semibold tracking-tight text-ink ${className ?? ""}`}
        style={{ fontSize: Math.max(8, Math.round(size * 0.3)) }}
        aria-hidden="true"
      >
        {BRAND_MONOGRAMS[id] ?? "?"}
      </span>
    )
  }

  return (
    <Image
      src={src}
      alt=""
      width={size}
      height={size}
      unoptimized
      className={`object-contain ${className ?? ""}`}
      aria-hidden="true"
    />
  )
}
