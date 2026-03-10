import { clsx, type ClassValue } from "clsx"
import { twMerge } from "tailwind-merge"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

export function formatNGN(amount: number | string): string {
  const num = typeof amount === 'string' ? parseFloat(amount) : amount
  if (isNaN(num)) return '₦0.00'
  return new Intl.NumberFormat('en-NG', {
    style: 'currency',
    currency: 'NGN',
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(num)
}

export function formatZEC(amount: string): string {
  // Always treat ZEC amounts as strings to preserve precision
  const parts = amount.split('.')
  if (parts.length === 1) return `${amount}.00000000`
  const decimals = parts[1].padEnd(8, '0').slice(0, 8)
  return `${parts[0]}.${decimals}`
}