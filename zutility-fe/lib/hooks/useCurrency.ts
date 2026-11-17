"use client"

import * as React from "react"
import { useAuthStore } from "@/store/auth"
import { DEFAULT_CURRENCY, detectCurrency, type CurrencyCode } from "@/lib/currency"

const subscribeToNothing = () => () => {}
const serverCurrency = () => DEFAULT_CURRENCY

export function useCurrency(initial?: CurrencyCode | null): CurrencyCode {
  const preferredCurrency = useAuthStore((s) => s.preferredCurrency)
  const userCurrency = useAuthStore((s) => s.user?.preferred_currency ?? null)
  const detected = React.useSyncExternalStore(subscribeToNothing, detectCurrency, serverCurrency)

  return preferredCurrency ?? userCurrency ?? initial ?? detected
}
