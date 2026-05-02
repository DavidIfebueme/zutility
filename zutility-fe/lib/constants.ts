export type UtilityType = 'airtime' | 'data' | 'tv' | 'electricity' | 'education' | 'school'

export interface UtilityVariation {
  variation_code: string
  name: string
  amount: number | null
}

export interface Utility {
  id: string
  type: UtilityType
  slug: string
  name: string
  descriptor: string
  iconType: string
  serviceRefLabel: string
  serviceRefPlaceholder: string
  serviceRefPattern?: string
  hasVariations: boolean
  hasAmountPicker: boolean
  amountMinKobo?: number
  amountMaxKobo?: number
  fixedAmountKobo?: number
  quickAmounts?: number[]
}

export const UTILITIES: Utility[] = [
  { id: 'mtn', type: 'airtime', slug: 'mtn', name: 'MTN Airtime', descriptor: 'Instant recharge', iconType: 'airtime', serviceRefLabel: 'Phone Number', serviceRefPlaceholder: '08012345678', serviceRefPattern: '^0[7-9][0-1]\\d{8}$', hasVariations: false, hasAmountPicker: true, amountMinKobo: 5000, amountMaxKobo: 5000000, quickAmounts: [500, 1000, 2000, 5000] },
  { id: 'airtel', type: 'airtime', slug: 'airtel', name: 'Airtel Airtime', descriptor: 'Instant recharge', iconType: 'airtime', serviceRefLabel: 'Phone Number', serviceRefPlaceholder: '08012345678', serviceRefPattern: '^0[7-9][0-1]\\d{8}$', hasVariations: false, hasAmountPicker: true, amountMinKobo: 5000, amountMaxKobo: 5000000, quickAmounts: [500, 1000, 2000, 5000] },
  { id: 'glo', type: 'airtime', slug: 'glo', name: 'Glo Airtime', descriptor: 'Instant recharge', iconType: 'airtime', serviceRefLabel: 'Phone Number', serviceRefPlaceholder: '08012345678', serviceRefPattern: '^0[7-9][0-1]\\d{8}$', hasVariations: false, hasAmountPicker: true, amountMinKobo: 5000, amountMaxKobo: 5000000, quickAmounts: [500, 1000, 2000, 5000] },
  { id: '9mobile', type: 'airtime', slug: '9mobile', name: '9mobile Airtime', descriptor: 'Instant recharge', iconType: 'airtime', serviceRefLabel: 'Phone Number', serviceRefPlaceholder: '08012345678', serviceRefPattern: '^0[7-9][0-1]\\d{8}$', hasVariations: false, hasAmountPicker: true, amountMinKobo: 5000, amountMaxKobo: 5000000, quickAmounts: [500, 1000, 2000, 5000] },
  { id: 'mtn-data', type: 'data', slug: 'mtn-data', name: 'MTN Data', descriptor: 'Data bundles', iconType: 'data', serviceRefLabel: 'Phone Number', serviceRefPlaceholder: '08012345678', serviceRefPattern: '^0[7-9][0-1]\\d{8}$', hasVariations: true, hasAmountPicker: false },
  { id: 'airtel-data', type: 'data', slug: 'airtel-data', name: 'Airtel Data', descriptor: 'Data bundles', iconType: 'data', serviceRefLabel: 'Phone Number', serviceRefPlaceholder: '08012345678', serviceRefPattern: '^0[7-9][0-1]\\d{8}$', hasVariations: true, hasAmountPicker: false },
  { id: 'glo-data', type: 'data', slug: 'glo-data', name: 'Glo Data', descriptor: 'Data bundles', iconType: 'data', serviceRefLabel: 'Phone Number', serviceRefPlaceholder: '08012345678', serviceRefPattern: '^0[7-9][0-1]\\d{8}$', hasVariations: true, hasAmountPicker: false },
  { id: '9mobile-data', type: 'data', slug: '9mobile-data', name: '9mobile Data', descriptor: 'Data bundles', iconType: 'data', serviceRefLabel: 'Phone Number', serviceRefPlaceholder: '08012345678', serviceRefPattern: '^0[7-9][0-1]\\d{8}$', hasVariations: true, hasAmountPicker: false },
  { id: 'dstv', type: 'tv', slug: 'dstv', name: 'DSTV', descriptor: 'Subscription renewal', iconType: 'tv', serviceRefLabel: 'Smartcard Number', serviceRefPlaceholder: 'e.g. 2012345678', serviceRefPattern: '^\\d{10}$', hasVariations: true, hasAmountPicker: false },
  { id: 'gotv', type: 'tv', slug: 'gotv', name: 'GOtv', descriptor: 'Subscription renewal', iconType: 'tv', serviceRefLabel: 'Smartcard Number', serviceRefPlaceholder: 'e.g. 2012345678', serviceRefPattern: '^\\d{10}$', hasVariations: true, hasAmountPicker: false },
  { id: 'startimes', type: 'tv', slug: 'startimes', name: 'Startimes', descriptor: 'Subscription renewal', iconType: 'tv', serviceRefLabel: 'Smartcard Number', serviceRefPlaceholder: 'e.g. 2012345678', serviceRefPattern: '^\\d{10}$', hasVariations: true, hasAmountPicker: false },
  { id: 'showmax', type: 'tv', slug: 'showmax', name: 'Showmax', descriptor: 'Streaming subscription', iconType: 'tv', serviceRefLabel: 'Email or Phone', serviceRefPlaceholder: 'you@email.com', hasVariations: true, hasAmountPicker: false },
  { id: 'ikeja-electric', type: 'electricity', slug: 'ikeja-electric', name: 'Ikeja Electric', descriptor: 'IKEDC prepaid token', iconType: 'electricity', serviceRefLabel: 'Meter Number', serviceRefPlaceholder: 'e.g. 54012345678', serviceRefPattern: '^\\d{11,13}$', hasVariations: false, hasAmountPicker: true, amountMinKobo: 50000, amountMaxKobo: 50000000, quickAmounts: [1000, 2000, 5000, 10000] },
  { id: 'eko-electric', type: 'electricity', slug: 'eko-electric', name: 'Eko Electric', descriptor: 'EKEDC prepaid token', iconType: 'electricity', serviceRefLabel: 'Meter Number', serviceRefPlaceholder: 'e.g. 54012345678', serviceRefPattern: '^\\d{11,13}$', hasVariations: false, hasAmountPicker: true, amountMinKobo: 50000, amountMaxKobo: 50000000, quickAmounts: [1000, 2000, 5000, 10000] },
  { id: 'abuja-electric', type: 'electricity', slug: 'abuja-electric', name: 'Abuja Electric', descriptor: 'AEDC prepaid token', iconType: 'electricity', serviceRefLabel: 'Meter Number', serviceRefPlaceholder: 'e.g. 54012345678', serviceRefPattern: '^\\d{11,13}$', hasVariations: false, hasAmountPicker: true, amountMinKobo: 50000, amountMaxKobo: 50000000, quickAmounts: [1000, 2000, 5000, 10000] },
  { id: 'ibadan-electric', type: 'electricity', slug: 'ibadan-electric', name: 'Ibadan Electric', descriptor: 'IBEDC prepaid token', iconType: 'electricity', serviceRefLabel: 'Meter Number', serviceRefPlaceholder: 'e.g. 54012345678', serviceRefPattern: '^\\d{11,13}$', hasVariations: false, hasAmountPicker: true, amountMinKobo: 50000, amountMaxKobo: 50000000, quickAmounts: [1000, 2000, 5000, 10000] },
  { id: 'kano-electric', type: 'electricity', slug: 'kano-electric', name: 'Kano Electric', descriptor: 'KEDCO prepaid token', iconType: 'electricity', serviceRefLabel: 'Meter Number', serviceRefPlaceholder: 'e.g. 54012345678', serviceRefPattern: '^\\d{11,13}$', hasVariations: false, hasAmountPicker: true, amountMinKobo: 50000, amountMaxKobo: 50000000, quickAmounts: [1000, 2000, 5000, 10000] },
  { id: 'phed-electric', type: 'electricity', slug: 'phed-electric', name: 'PH Electric', descriptor: 'PHED prepaid token', iconType: 'electricity', serviceRefLabel: 'Meter Number', serviceRefPlaceholder: 'e.g. 54012345678', serviceRefPattern: '^\\d{11,13}$', hasVariations: false, hasAmountPicker: true, amountMinKobo: 50000, amountMaxKobo: 50000000, quickAmounts: [1000, 2000, 5000, 10000] },
  { id: 'jos-electric', type: 'electricity', slug: 'jos-electric', name: 'Jos Electric', descriptor: 'JED prepaid token', iconType: 'electricity', serviceRefLabel: 'Meter Number', serviceRefPlaceholder: 'e.g. 54012345678', serviceRefPattern: '^\\d{11,13}$', hasVariations: false, hasAmountPicker: true, amountMinKobo: 50000, amountMaxKobo: 50000000, quickAmounts: [1000, 2000, 5000, 10000] },
  { id: 'kaduna-electric', type: 'electricity', slug: 'kaduna-electric', name: 'Kaduna Electric', descriptor: 'KAEDCO prepaid token', iconType: 'electricity', serviceRefLabel: 'Meter Number', serviceRefPlaceholder: 'e.g. 54012345678', serviceRefPattern: '^\\d{11,13}$', hasVariations: false, hasAmountPicker: true, amountMinKobo: 50000, amountMaxKobo: 50000000, quickAmounts: [1000, 2000, 5000, 10000] },
  { id: 'enugu-electric', type: 'electricity', slug: 'enugu-electric', name: 'Enugu Electric', descriptor: 'EEDC prepaid token', iconType: 'electricity', serviceRefLabel: 'Meter Number', serviceRefPlaceholder: 'e.g. 54012345678', serviceRefPattern: '^\\d{11,13}$', hasVariations: false, hasAmountPicker: true, amountMinKobo: 50000, amountMaxKobo: 50000000, quickAmounts: [1000, 2000, 5000, 10000] },
  { id: 'benin-electric', type: 'electricity', slug: 'benin-electric', name: 'Benin Electric', descriptor: 'BEDC prepaid token', iconType: 'electricity', serviceRefLabel: 'Meter Number', serviceRefPlaceholder: 'e.g. 54012345678', serviceRefPattern: '^\\d{11,13}$', hasVariations: false, hasAmountPicker: true, amountMinKobo: 50000, amountMaxKobo: 50000000, quickAmounts: [1000, 2000, 5000, 10000] },
  { id: 'yola-electric', type: 'electricity', slug: 'yola-electric', name: 'Yola Electric', descriptor: 'YEDC prepaid token', iconType: 'electricity', serviceRefLabel: 'Meter Number', serviceRefPlaceholder: 'e.g. 54012345678', serviceRefPattern: '^\\d{11,13}$', hasVariations: false, hasAmountPicker: true, amountMinKobo: 50000, amountMaxKobo: 50000000, quickAmounts: [1000, 2000, 5000, 10000] },
  { id: 'aba-electric', type: 'electricity', slug: 'aba-electric', name: 'Aba Electric', descriptor: 'ABA prepaid token', iconType: 'electricity', serviceRefLabel: 'Meter Number', serviceRefPlaceholder: 'e.g. 54012345678', serviceRefPattern: '^\\d{11,13}$', hasVariations: false, hasAmountPicker: true, amountMinKobo: 50000, amountMaxKobo: 50000000, quickAmounts: [1000, 2000, 5000, 10000] },
  { id: 'waec-registration', type: 'education', slug: 'waec-registration', name: 'WAEC Registration', descriptor: 'Exam registration PIN', iconType: 'education', serviceRefLabel: 'Phone Number', serviceRefPlaceholder: '08012345678', serviceRefPattern: '^0[7-9][0-1]\\d{8}$', hasVariations: true, hasAmountPicker: false },
  { id: 'waec-result-checker', type: 'education', slug: 'waec-result-checker', name: 'WAEC Result Checker', descriptor: 'Result checker PIN', iconType: 'education', serviceRefLabel: 'Phone Number', serviceRefPlaceholder: '08012345678', serviceRefPattern: '^0[7-9][0-1]\\d{8}$', hasVariations: true, hasAmountPicker: false },
  { id: 'jamb', type: 'education', slug: 'jamb', name: 'JAMB Pin', descriptor: 'UTME registration PIN', iconType: 'education', serviceRefLabel: 'Phone Number', serviceRefPlaceholder: '08012345678', serviceRefPattern: '^0[7-9][0-1]\\d{8}$', hasVariations: false, hasAmountPicker: false, fixedAmountKobo: 650000 },
  { id: 'school-fees', type: 'school', slug: 'school-fees', name: 'School Fees', descriptor: 'Pay via RRR or Student ID', iconType: 'school', serviceRefLabel: 'RRR or Student ID', serviceRefPlaceholder: 'Enter RRR or Student ID', hasVariations: false, hasAmountPicker: true, amountMinKobo: 50000, amountMaxKobo: 500000000, quickAmounts: [10000, 25000, 50000, 100000] },
]

export const UTILITY_CATEGORIES: { label: string; type: UtilityType | 'school'; icon: string }[] = [
  { label: 'Airtime', type: 'airtime', icon: 'airtime' },
  { label: 'Data', type: 'data', icon: 'data' },
  { label: 'TV', type: 'tv', icon: 'tv' },
  { label: 'Electricity', type: 'electricity', icon: 'electricity' },
  { label: 'Education', type: 'education', icon: 'education' },
  { label: 'School Fees', type: 'school', icon: 'school' },
]

export const TOKENS = [
  { id: 'ZEC', name: 'Zcash', symbol: 'ZEC', status: 'live' },
  { id: 'COMING_SOON_1', name: 'Coming Soon', symbol: 'SOON', status: 'coming_soon' },
  { id: 'COMING_SOON_2', name: 'Coming Soon', symbol: 'SOON', status: 'coming_soon' },
  { id: 'COMING_SOON_3', name: 'Coming Soon', symbol: 'SOON', status: 'coming_soon' },
]
