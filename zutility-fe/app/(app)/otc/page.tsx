"use client"

import Link from "next/link"
import ArrowBackUpIcon from "@/components/icons/arrow-back-up-icon"
import ArrowNarrowRightIcon from "@/components/icons/arrow-narrow-right-icon"
import { Button } from "@/components/ui/button"

export default function OtcPage() {
  return (
    <div className="flex flex-col items-center justify-center min-h-[60vh] text-center px-4">
      <div className="h-20 w-20 rounded-2xl bg-accent-zec/10 flex items-center justify-center mb-6">
        <ArrowBackUpIcon size={40} className="text-accent-zec" />
      </div>
      <h1 className="font-dela text-3xl tracking-tight mb-3">OTC Off-ramp</h1>
      <p className="text-text-secondary max-w-md mb-2">
        Swap your ZEC directly to local currency at competitive rates. No middleman, no hassle.
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
