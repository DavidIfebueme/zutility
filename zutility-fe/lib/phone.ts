const MOBILE_PREFIXES = ["070", "071", "080", "081", "090", "091"]

export function normalizePhone(raw: string): string | null {
  const digits = raw.replace(/[\s().-]/g, "")
  const national = digits.replace(/^\+/, "").replace(/^234/, "")
  const local = national.length === 10 ? `0${national}` : national

  if (!/^\d{11}$/.test(local)) return null
  if (!MOBILE_PREFIXES.includes(local.slice(0, 3))) return null

  return local
}
