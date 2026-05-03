import { clsx, type ClassValue } from "clsx"
import { twMerge } from "tailwind-merge"
import { formatCurrency, type CurrencyCode } from "@/lib/currency"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

export function formatNGN(amount: number | string): string {
  return formatCurrency(amount, 'NGN')
}

export function formatLocal(amount: number | string, currency: CurrencyCode = 'NGN'): string {
  return formatCurrency(amount, currency)
}

export function formatZEC(amount: string): string {
  const parts = amount.split('.')
  if (parts.length === 1) return `${amount}.00000000`
  const decimals = parts[1].padEnd(8, '0').slice(0, 8)
  return `${parts[0]}.${decimals}`
}