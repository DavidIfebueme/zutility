import * as React from "react"
import { useRate } from "./useRate"
import type { FxRates } from "@/lib/currency"

export function useFxRates(): FxRates | null {
  const { rate } = useRate()

  return React.useMemo(
    () =>
      rate
        ? {
            usd_ngn: parseFloat(rate.usd_ngn) || 1,
            usd_kes: parseFloat(rate.usd_kes) || 0,
            usd_ghs: parseFloat(rate.usd_ghs) || 0,
            usd_zar: parseFloat(rate.usd_zar) || 0,
            usd_egp: parseFloat(rate.usd_egp) || 0,
          }
        : null,
    [rate]
  )
}
