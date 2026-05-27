"use client"

import Link from "next/link"
import ShoppingCartIcon from "@/components/icons/shopping-cart-icon"
import ArrowNarrowRightIcon from "@/components/icons/arrow-narrow-right-icon"
import { Button } from "@/components/ui/button"

export default function P2pPage() {
  return (
    <div className="flex flex-col items-center justify-center min-h-[60vh] text-center px-4">
      <div className="h-20 w-20 rounded-2xl bg-accent-zec/10 flex items-center justify-center mb-6">
        <ShoppingCartIcon size={40} className="text-accent-zec" />
      </div>
      <h1 className="font-dela text-3xl tracking-tight mb-3">P2P Marketplace</h1>
      <p className="text-text-secondary max-w-md mb-2">
        Buy and sell ZEC peer-to-peer with escrow protection. Set your own price, trade on your terms.
      </p>
      <p className="text-accent-zec font-medium mb-8">Coming Soon</p>
      <Link href="/dashboard">
        <Button variant="secondary">
          Back to Dashboard <ArrowNarrowRightIcon size={16} className="ml-2" />
        </Button>
      </Link>
    </div>
  )
}
