"use client"

import { useAuthStore } from "@/store/auth"
import { detectCurrency, type CurrencyCode } from "@/lib/currency"

export function useCurrency(): CurrencyCode {
  const preferredCurrency = useAuthStore((s) => s.preferredCurrency)
  return preferredCurrency || detectCurrency()
}
