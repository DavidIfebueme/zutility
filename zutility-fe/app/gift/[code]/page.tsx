import type { Metadata } from "next"
import { cache } from "react"
import { formatZEC } from "@/lib/utils"
import { billerBySlug } from "@/lib/catalogue"
import type { CatalogueResponse, GiftView } from "@/lib/types"
import { GiftClaim } from "./gift-claim"

const apiBase = process.env.NEXT_PUBLIC_API_URL?.replace(/\/$/, "") || "http://127.0.0.1:3001"

const fetchGift = cache(async (code: string): Promise<GiftView | null> => {
  try {
    const response = await fetch(`${apiBase}/api/v1/gifts/${encodeURIComponent(code)}`, {
      cache: "no-store",
    })
    if (!response.ok) return null
    return (await response.json()) as GiftView
  } catch {
    return null
  }
})

const fetchCatalogue = cache(async (): Promise<CatalogueResponse | null> => {
  try {
    const response = await fetch(`${apiBase}/api/v1/catalogue`, { next: { revalidate: 300 } })
    if (!response.ok) return null
    return (await response.json()) as CatalogueResponse
  } catch {
    return null
  }
})

export async function generateMetadata({
  params,
}: {
  params: Promise<{ code: string }>
}): Promise<Metadata> {
  const { code } = await params
  const gift = await fetchGift(code)

  if (!gift) {
    return { title: "Gift not found | zutility" }
  }

  const amount = gift.zec_amount ? `${formatZEC(gift.zec_amount)} ZEC` : "a utility gift"
  const title = `${gift.sender_name} sent you ${amount}`
  const description = gift.message ?? "Claim your utility gift on zutility."

  return {
    title: `${title} | zutility`,
    description,
    openGraph: { title, description, type: "website" },
    twitter: { card: "summary", title, description },
  }
}

export default async function GiftPage({ params }: { params: Promise<{ code: string }> }) {
  const { code } = await params
  const [gift, catalogue] = await Promise.all([fetchGift(code), fetchCatalogue()])
  const biller = gift ? billerBySlug(catalogue, gift.utility_slug) : null
  return <GiftClaim code={code} initialGift={gift} biller={biller} />
}
