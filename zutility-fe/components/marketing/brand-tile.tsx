"use client"

import * as React from "react"
import { getBrandIcon } from "@/lib/brand-icon-map"
import type { BrandIconId } from "@/lib/constants"

export function BrandTile({ id, size = 24 }: { id: BrandIconId; size?: number }) {
  return (
    <span className="flex h-10 w-10 items-center justify-center rounded-md bg-paper-2 text-ink">
      {React.createElement(getBrandIcon(id), { size })}
    </span>
  )
}
