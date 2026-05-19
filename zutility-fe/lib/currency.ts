export type CurrencyCode = 'NGN' | 'KES' | 'GHS' | 'ZAR' | 'EGP' | 'USD'

export interface CurrencyInfo {
  code: CurrencyCode
  symbol: string
  locale: string
  name: string
  country: string
}

export const CURRENCIES: Record<CurrencyCode, CurrencyInfo> = {
  NGN: { code: 'NGN', symbol: '₦', locale: 'en-NG', name: 'Naira', country: 'Nigeria' },
  KES: { code: 'KES', symbol: 'KSh', locale: 'en-KE', name: 'Shilling', country: 'Kenya' },
  GHS: { code: 'GHS', symbol: 'GH₵', locale: 'en-GH', name: 'Cedi', country: 'Ghana' },
  ZAR: { code: 'ZAR', symbol: 'R', locale: 'en-ZA', name: 'Rand', country: 'South Africa' },
  EGP: { code: 'EGP', symbol: 'E£', locale: 'ar-EG', name: 'Pound', country: 'Egypt' },
  USD: { code: 'USD', symbol: '$', locale: 'en-US', name: 'Dollar', country: 'Global' },
}

const TIMEZONE_CURRENCY: Record<string, CurrencyCode> = {
  'Africa/Lagos': 'NGN',
  'Africa/Porto-Novo': 'NGN',
  'Africa/Douala': 'NGN',
  'Africa/Malabo': 'NGN',
  'Africa/Libreville': 'NGN',
  'Africa/Niamey': 'NGN',
  'Africa/Bangui': 'NGN',
  'Africa/Nairobi': 'KES',
  'Africa/Addis_Ababa': 'KES',
  'Africa/Dar_es_Salaam': 'KES',
  'Africa/Kampala': 'KES',
  'Africa/Mogadishu': 'KES',
  'Africa/Asmara': 'KES',
  'Africa/Accra': 'GHS',
  'Africa/Abidjan': 'GHS',
  'Africa/Ouagadougou': 'GHS',
  'Africa/Bamako': 'GHS',
  'Africa/Conakry': 'GHS',
  'Africa/Freetown': 'GHS',
  'Africa/Monrovia': 'GHS',
  'Africa/Lome': 'GHS',
  'Africa/Ndjamena': 'GHS',
  'Africa/Dakar': 'GHS',
  'Africa/Banjul': 'GHS',
  'Africa/Bissau': 'GHS',
  'Africa/Nouakchott': 'GHS',
  'Africa/Johannesburg': 'ZAR',
  'Africa/Maseru': 'ZAR',
  'Africa/Mbabane': 'ZAR',
  'Africa/Maputo': 'ZAR',
  'Africa/Gaborone': 'ZAR',
  'Africa/Windhoek': 'ZAR',
  'Africa/Harare': 'ZAR',
  'Africa/Lusaka': 'ZAR',
  'Africa/Blantyre': 'ZAR',
  'Africa/Cairo': 'EGP',
  'Africa/Tripoli': 'EGP',
  'Africa/Khartoum': 'EGP',
  'Africa/Tunis': 'EGP',
  'Africa/Algiers': 'EGP',
  'Africa/Casablanca': 'EGP',
}

export function detectCurrency(): CurrencyCode {
  if (typeof window === 'undefined') return 'USD'
  try {
    const tz = Intl.DateTimeFormat().resolvedOptions().timeZone
    return TIMEZONE_CURRENCY[tz] || 'USD'
  } catch {
    return 'USD'
  }
}

export function formatCurrency(amount: number | string, currency: CurrencyCode = 'NGN'): string {
  const num = typeof amount === 'string' ? parseFloat(amount) : amount
  if (isNaN(num)) return `${CURRENCIES[currency].symbol}0.00`
  try {
    return new Intl.NumberFormat(CURRENCIES[currency].locale, {
      style: 'currency',
      currency,
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    }).format(num)
  } catch {
    return `${CURRENCIES[currency].symbol}${num.toFixed(2)}`
  }
}

export interface FxRates {
  usd_ngn: number
  usd_kes: number
  usd_ghs: number
  usd_zar: number
  usd_egp: number
}

export function convertFromNGN(amountNGN: number, targetCurrency: CurrencyCode, rates: FxRates): number {
  if (targetCurrency === 'NGN') return amountNGN
  const usdAmount = amountNGN / (rates.usd_ngn || 1)
  switch (targetCurrency) {
    case 'KES': return usdAmount * (rates.usd_kes || 0)
    case 'GHS': return usdAmount * (rates.usd_ghs || 0)
    case 'ZAR': return usdAmount * (rates.usd_zar || 0)
    case 'EGP': return usdAmount * (rates.usd_egp || 0)
    case 'USD': return usdAmount
    default: return amountNGN
  }
}

export function convertToNGN(amountLocal: number, fromCurrency: CurrencyCode, rates: FxRates): number {
  if (fromCurrency === 'NGN') return amountLocal
  let usdAmount: number
  switch (fromCurrency) {
    case 'KES': usdAmount = amountLocal / (rates.usd_kes || 1); break
    case 'GHS': usdAmount = amountLocal / (rates.usd_ghs || 1); break
    case 'ZAR': usdAmount = amountLocal / (rates.usd_zar || 1); break
    case 'EGP': usdAmount = amountLocal / (rates.usd_egp || 1); break
    case 'USD': usdAmount = amountLocal; break
    default: return amountLocal
  }
  return usdAmount * (rates.usd_ngn || 1)
}

export function formatLocalAmount(amountNGN: number, currency: CurrencyCode, rates: FxRates): string {
  if (currency === 'NGN') return formatCurrency(amountNGN, 'NGN')
  const converted = convertFromNGN(amountNGN, currency, rates)
  if (converted === 0) return formatCurrency(amountNGN, 'NGN')
  return formatCurrency(converted, currency)
}
