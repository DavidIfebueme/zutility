import type { BrandIconId } from '@/lib/constants'

export interface SampleOrder {
  id: string
  utility: string
  detail: string
  amountNgn: number
  zec: string
  address: string
  brand: BrandIconId
}

export const SAMPLE_ORDERS: SampleOrder[] = [
  {
    id: "8F2C-41A9",
    utility: "MTN Airtime",
    detail: "0803 555 0142",
    amountNgn: 2000,
    zec: "0.00153524",
    address: "utest1q9x4v7k2m8p3n6r5t0w1y2z3a4b5c6d7e8f9g0h",
    brand: "mtn",
  },
  {
    id: "2B7D-90C4",
    utility: "Ikeja Electric",
    detail: "Meter 0412 3456 789",
    amountNgn: 10000,
    zec: "0.00767618",
    address: "utest1h7k2m8p3n6r5t0w1y2z3a4b5c6d7e8f9g0h1j2k3l4m5n6",
    brand: "ikeja-electric",
  },
  {
    id: "5A1E-73B8",
    utility: "School Fees",
    detail: "UNN · RRR 141523991664",
    amountNgn: 52000,
    zec: "0.03991616",
    address: "utest1m8p3n6r5t0w1y2z3a4b5c6d7e8f9g0h1j2k3l4m5n6p7q",
    brand: "unn",
  },
  {
    id: "C3F6-2D10",
    utility: "DStv Compact",
    detail: "Smartcard 7021 345 678",
    amountNgn: 19000,
    zec: "0.01458475",
    address: "utest1n6r5t0w1y2z3a4b5c6d7e8f9g0h1j2k3l4m5n6p7q8r9s",
    brand: "dstv",
  },
  {
    id: "9D4B-55E2",
    utility: "Airtel Data",
    detail: "10GB · 0802 555 0142",
    amountNgn: 5000,
    zec: "0.00383809",
    address: "utest1p3n6r5t0w1y2z3a4b5c6d7e8f9g0h1j2k3l4m5n6p7q8r9t",
    brand: "airtel",
  },
]
