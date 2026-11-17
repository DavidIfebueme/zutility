import { clsx, type ClassValue } from "clsx"
import { twMerge } from "tailwind-merge"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

export function formatZEC(amount: string): string {
  const parts = amount.split('.')
  if (parts.length === 1) return `${amount}.00000000`
  const decimals = parts[1].padEnd(8, '0').slice(0, 8)
  return `${parts[0]}.${decimals}`
}
