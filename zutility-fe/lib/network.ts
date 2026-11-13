export type MobileNetwork = "MTN" | "GLO" | "AIRTEL" | "9MOBILE"

const PREFIX_NETWORK: Record<string, MobileNetwork> = {
  "0803": "MTN", "0806": "MTN", "0703": "MTN", "0706": "MTN",
  "0813": "MTN", "0816": "MTN", "0810": "MTN", "0814": "MTN",
  "0903": "MTN", "0906": "MTN", "0913": "MTN", "0916": "MTN",
  "0704": "MTN",
  "0805": "GLO", "0807": "GLO", "0705": "GLO", "0815": "GLO",
  "0811": "GLO", "0905": "GLO", "0915": "GLO",
  "0802": "AIRTEL", "0808": "AIRTEL", "0708": "AIRTEL", "0812": "AIRTEL",
  "0701": "AIRTEL", "0902": "AIRTEL", "0901": "AIRTEL", "0904": "AIRTEL",
  "0907": "AIRTEL", "0912": "AIRTEL",
  "0809": "9MOBILE", "0817": "9MOBILE", "0818": "9MOBILE",
  "0908": "9MOBILE", "0909": "9MOBILE",
}

const SLUG_NETWORK: Record<string, MobileNetwork> = {
  mtn: "MTN", "mtn-data": "MTN",
  glo: "GLO", "glo-data": "GLO",
  airtel: "AIRTEL", "airtel-data": "AIRTEL",
  "9mobile": "9MOBILE", "9mobile-data": "9MOBILE",
}

export function networkForSlug(slug: string): MobileNetwork | null {
  return SLUG_NETWORK[slug] ?? null
}

export function networkForNumber(input: string): MobileNetwork | null {
  const digits = input.replace(/\D/g, "")
  const local = digits.startsWith("234") ? `0${digits.slice(3)}` : digits
  return PREFIX_NETWORK[local.slice(0, 4)] ?? null
}

export function networkWarning(serviceRef: string, slug: string): string | null {
  const selected = networkForSlug(slug)
  const detected = networkForNumber(serviceRef)
  if (!selected || !detected || selected === detected) return null
  return `This looks like a ${detected} number, but you selected ${selected}. Ported numbers can change network, so continue if it is correct.`
}
