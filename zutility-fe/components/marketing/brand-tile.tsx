"use client"

import type { BrandIconId } from "@/lib/constants"
import { BrandMark } from "@/components/ui/brand-mark"
import { BRAND_TILE_BACKGROUND } from "@/lib/brand-logo"

export function BrandTile({ id, size = 26 }: { id: BrandIconId; size?: number }) {
  return (
    <span
      className="flex h-10 w-10 items-center justify-center rounded-md bg-paper-2"
      style={BRAND_TILE_BACKGROUND[id] ? { backgroundColor: BRAND_TILE_BACKGROUND[id] } : undefined}
    >
      <BrandMark id={id} size={size} />
    </span>
  )
}
