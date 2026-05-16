"use client"

import * as React from "react"
import Link from "next/link"
import { useRouter, useSearchParams } from "next/navigation"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import * as z from "zod"
import { Suspense } from "react"
import { motion } from "motion/react"
import { Mail, User, ArrowRight, Copy, Check, Users, Shield, Zap, Clock } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card"
import { apiPost, apiGet } from "@/lib/api"
import { toast } from "sonner"
import dynamic from "next/dynamic"

const ZecCoinScene = dynamic(() => import("@/components/3d/ZecCoin").then(mod => mod.ZecCoinScene), { ssr: false })
const Canvas = dynamic(() => import("@react-three/fiber").then(mod => mod.Canvas), { ssr: false })

const waitlistSchema = z.object({
  email: z.string().email("Invalid email address"),
  displayName: z.string().optional(),
})

type WaitlistFormValues = z.infer<typeof waitlistSchema>

interface WaitlistJoinResponse {
  referral_code: string
  position: number
}

interface WaitlistStats {
  total: number
  verified: number
}

function WaitlistContent() {
  const router = useRouter()
  const searchParams = useSearchParams()
  const refCode = searchParams.get("ref")
  const [isLoading, setIsLoading] = React.useState(false)
  const [joined, setJoined] = React.useState<WaitlistJoinResponse | null>(null)
  const [copied, setCopied] = React.useState(false)
  const [stats, setStats] = React.useState<WaitlistStats | null>(null)

  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<WaitlistFormValues>({
    resolver: zodResolver(waitlistSchema),
  })

  React.useEffect(() => {
    apiGet<WaitlistStats>("/api/v1/waitlist/stats").then(setStats).catch(() => {})
  }, [])

  const onSubmit = async (data: WaitlistFormValues) => {
    setIsLoading(true)
    try {
      const utmParams: Record<string, string> = {}
      for (const key of ["utm_source", "utm_medium", "utm_campaign", "utm_content", "utm_term"]) {
        const val = searchParams.get(key)
        if (val) utmParams[key] = val
      }

      const result = await apiPost<WaitlistJoinResponse>("/api/v1/waitlist/join", {
        email: data.email,
        display_name: data.displayName || undefined,
        ref_code: refCode || undefined,
        ...utmParams,
      })
      setJoined(result)
      toast.success("You're on the list!")
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : "Failed to join waitlist"
      toast.error(message)
    } finally {
      setIsLoading(false)
    }
  }

  const shareUrl = typeof window !== "undefined" && joined
    ? `${window.location.origin}/waitlist?ref=${joined.referral_code}`
    : ""

  const handleCopy = async () => {
    if (!shareUrl) return
    await navigator.clipboard.writeText(shareUrl)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  return (
    <div className="flex min-h-screen bg-bg-void text-text-primary">
      <div className="hidden w-1/2 flex-col justify-between border-r border-border-subtle bg-bg-surface p-12 lg:flex relative overflow-hidden">
        <div className="absolute inset-0 z-0 opacity-30 pointer-events-none">
          <Canvas camera={{ position: [0, 0, 5], fov: 45 }}>
            <ambientLight intensity={0.5} />
            <ZecCoinScene />
          </Canvas>
        </div>
        <div className="relative z-10">
          <Link href="/" className="font-dela text-2xl tracking-tight">
            <span className="text-accent-zec">z</span>utility
          </Link>
        </div>
        <div className="relative z-10 max-w-md">
          <h1 className="font-dela text-4xl leading-tight mb-4">
            Be first in line.
          </h1>
          <p className="text-text-secondary text-lg">
            Join the waitlist for early access to private, ZEC-powered utility payments across Africa.
          </p>
          {stats && stats.total > 0 && (
            <div className="mt-8 flex items-center gap-3">
              <div className="flex -space-x-2">
                {[...Array(3)].map((_, i) => (
                  <div key={i} className="h-8 w-8 rounded-full bg-accent-zec/20 border-2 border-bg-surface flex items-center justify-center text-xs text-accent-zec font-medium">
                    {["Z", "U", "X"][i]}
                  </div>
                ))}
              </div>
              <span className="text-sm text-text-secondary">
                <span className="text-text-primary font-medium">{stats.verified}</span> people already verified
              </span>
            </div>
          )}
        </div>
        <div className="relative z-10">
          <div className="flex items-center gap-6 text-sm text-text-muted">
            <div className="flex items-center gap-2">
              <Shield className="h-4 w-4 text-accent-zec" />
              <span>No KYC</span>
            </div>
            <div className="flex items-center gap-2">
              <Zap className="h-4 w-4 text-accent-zec" />
              <span>Instant delivery</span>
            </div>
            <div className="flex items-center gap-2">
              <Clock className="h-4 w-4 text-accent-zec" />
              <span>3 confs ~4min</span>
            </div>
          </div>
        </div>
      </div>

      <div className="flex w-full flex-col justify-center px-8 sm:px-16 lg:w-1/2 xl:px-24">
        <div className="mx-auto w-full max-w-sm">
          <div className="mb-10 lg:hidden">
            <Link href="/" className="font-dela text-2xl tracking-tight">
              <span className="text-accent-zec">z</span>utility
            </Link>
          </div>

          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.4 }}
          >
            {joined ? (
              <div className="space-y-6">
                <div>
                  <h2 className="font-dela text-3xl mb-2">You&apos;re on the list!</h2>
                  <p className="text-text-secondary">
                    You&apos;re <span className="text-accent-zec font-semibold">#{joined.position}</span> in line. Check your email to confirm your spot.
                  </p>
                </div>

                <Card className="border-accent-zec/30 bg-bg-elevated">
                  <CardHeader className="pb-3">
                    <CardTitle className="text-base flex items-center gap-2">
                      <Users className="h-4 w-4 text-accent-zec" />
                      Your referral link
                    </CardTitle>
                    <CardDescription>Share to move up the list</CardDescription>
                  </CardHeader>
                  <CardContent>
                    <div className="flex items-center gap-2">
                      <code className="flex-1 rounded-md bg-bg-void px-3 py-2 text-xs text-accent-zec truncate border border-border-subtle">
                        {shareUrl}
                      </code>
                      <Button variant="secondary" size="sm" onClick={handleCopy} className="shrink-0">
                        {copied ? <Check className="h-4 w-4" /> : <Copy className="h-4 w-4" />}
                      </Button>
                    </div>
                    <p className="text-xs text-text-muted mt-3">
                      Your code: <span className="text-text-primary font-mono">{joined.referral_code}</span>
                    </p>
                  </CardContent>
                </Card>
              </div>
            ) : (
              <>
                <h2 className="font-dela text-3xl mb-2">Join the Waitlist</h2>
                <p className="text-text-secondary mb-8">
                  Get early access to private utility payments with Zcash.
                </p>

                <form onSubmit={handleSubmit(onSubmit)} className="space-y-5">
                  <div className="space-y-1">
                    <Input
                      {...register("email")}
                      type="email"
                      placeholder="Email address"
                      leftIcon={<Mail className="h-5 w-5" />}
                      error={errors.email?.message}
                    />
                  </div>

                  <div className="space-y-1">
                    <Input
                      {...register("displayName")}
                      type="text"
                      placeholder="Name (optional)"
                      leftIcon={<User className="h-5 w-5" />}
                      error={errors.displayName?.message}
                    />
                  </div>

                  <Button type="submit" className="w-full h-12 text-base mt-4" loading={isLoading}>
                    Join Waitlist <ArrowRight className="ml-2 h-4 w-4" />
                  </Button>

                  {refCode && (
                    <p className="text-xs text-text-muted text-center">
                      Referred by <span className="font-mono text-text-secondary">{refCode}</span>
                    </p>
                  )}
                </form>

                <p className="text-xs text-text-muted text-center mt-6">
                  We&apos;ll only send you waitlist updates. No spam.
                </p>
              </>
            )}
          </motion.div>
        </div>
      </div>
    </div>
  )
}

export default function WaitlistPage() {
  return (
    <Suspense fallback={<div className="flex min-h-screen items-center justify-center bg-bg-void text-text-muted">Loading...</div>}>
      <WaitlistContent />
    </Suspense>
  )
}
