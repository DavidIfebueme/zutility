"use client"

import * as React from "react"
import Link from "next/link"
import { useSearchParams } from "next/navigation"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import * as z from "zod"
import { Suspense } from "react"
import { ArrowRight, Check, Copy } from "lucide-react"
import { SiteHeader } from "@/components/marketing/site-header"
import { SiteFooter } from "@/components/marketing/site-footer"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { apiPost, apiGet } from "@/lib/api"

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
  const searchParams = useSearchParams()
  const refCode = searchParams.get("ref")
  const [isLoading, setIsLoading] = React.useState(false)
  const [joined, setJoined] = React.useState<WaitlistJoinResponse | null>(null)
  const [copied, setCopied] = React.useState(false)
  const [stats, setStats] = React.useState<WaitlistStats | null>(null)
  const [error, setError] = React.useState<string | null>(null)

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
    setError(null)
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
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : "Failed to join waitlist"
      setError(message)
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
    <div className="min-h-screen bg-paper">
      <SiteHeader />

      <main>
        <section className="paper-grid border-b border-line">
          <div className="mx-auto max-w-6xl px-5 py-14 lg:py-16">
            <p className="label-xs text-gold-deep">Waitlist</p>
            <h1 className="mt-4 max-w-2xl text-4xl leading-[1.1] text-ink sm:text-5xl">
              Be first in line.
            </h1>
            <p className="mt-4 max-w-xl text-[15px] leading-relaxed text-ink-2">
              Early access to private utility payments in Zcash. Leave an email, confirm it, and
              hold your place.
            </p>
          </div>
        </section>

        <section>
          <div className="mx-auto grid max-w-6xl gap-10 px-5 py-12 lg:grid-cols-[1fr_440px] lg:gap-16 lg:py-16">
            <div className="max-w-md">
              <p className="label-xs text-muted">How the list works</p>
              <dl className="mt-4 border-t border-line">
                <div className="border-b border-line py-4">
                  <dt className="text-[14px] font-medium text-ink">Join with an email</dt>
                  <dd className="mt-1 text-[13px] leading-relaxed text-ink-2">
                    The confirmation link is the only step. No documents, no account approval.
                  </dd>
                </div>
                <div className="border-b border-line py-4">
                  <dt className="text-[14px] font-medium text-ink">Confirm to hold the place</dt>
                  <dd className="mt-1 text-[13px] leading-relaxed text-ink-2">
                    An unconfirmed address is not counted as verified. Open the link in the message
                    to confirm it.
                  </dd>
                </div>
                <div className="border-b border-line py-4">
                  <dt className="text-[14px] font-medium text-ink">Referrals move you up</dt>
                  <dd className="mt-1 text-[13px] leading-relaxed text-ink-2">
                    Share your link after joining. Each person who joins through it helps your
                    position.
                  </dd>
                </div>
              </dl>

              {stats && stats.total > 0 && (
                <div className="mt-8">
                  <p className="label-xs text-muted">The list right now</p>
                  <dl className="mt-3 rounded-lg border border-line bg-cream">
                    <div className="flex items-baseline justify-between border-b border-line px-4 py-3">
                      <dt className="text-[13px] text-muted">Joined</dt>
                      <dd className="num text-[14px] text-ink">{stats.total}</dd>
                    </div>
                    <div className="flex items-baseline justify-between px-4 py-3">
                      <dt className="text-[13px] text-muted">Verified</dt>
                      <dd className="num text-[14px] text-ink">{stats.verified}</dd>
                    </div>
                  </dl>
                </div>
              )}

              <p className="mt-8 text-[13px] leading-relaxed text-muted">
                We send waitlist updates and nothing else. Read the{" "}
                <Link
                  href="/how-it-works"
                  className="text-gold-deep u-link"
                >
                  payment flow
                </Link>{" "}
                before you join.
              </p>
            </div>

            <div className="rounded-lg border border-line bg-cream p-6 sm:p-7">
              {joined ? (
                <div aria-live="polite">
                  <span className="flex h-10 w-10 items-center justify-center rounded-md bg-paper-2 text-green">
                    <Check className="h-5 w-5" strokeWidth={1.8} />
                  </span>
                  <h2 className="mt-4 text-2xl text-ink">You are on the list.</h2>
                  <p className="mt-2 text-[14px] leading-relaxed text-ink-2">
                    Check the email for the confirmation link. The place holds once that link is
                    opened.
                  </p>

                  <dl className="mt-6 rounded-md border border-line bg-paper">
                    <div className="flex items-baseline justify-between border-b border-line px-4 py-3">
                      <dt className="text-[13px] text-muted">Position</dt>
                      <dd className="num text-[15px] text-ink">#{joined.position}</dd>
                    </div>
                    <div className="flex items-baseline justify-between px-4 py-3">
                      <dt className="text-[13px] text-muted">Referral code</dt>
                      <dd className="num text-[13px] text-ink">{joined.referral_code}</dd>
                    </div>
                  </dl>

                  <div className="mt-5">
                    <p className="text-[13px] font-medium text-ink">Your referral link</p>
                    <div className="mt-2 flex items-stretch gap-2">
                      <code className="num min-w-0 flex-1 truncate rounded-md border border-line bg-paper px-3 py-2 text-[12px] text-ink-2">
                        {shareUrl}
                      </code>
                      <Button
                        variant="secondary"
                        size="sm"
                        onClick={handleCopy}
                        className="shrink-0"
                        aria-label="Copy referral link"
                      >
                        {copied ? (
                          <Check className="h-4 w-4 text-green" strokeWidth={1.8} />
                        ) : (
                          <Copy className="h-4 w-4" strokeWidth={1.6} />
                        )}
                      </Button>
                    </div>
                    <p className="mt-2 text-[12px] text-muted">
                      Share it with anyone who wants early access.
                    </p>
                  </div>

                  <div className="mt-6 border-t border-line pt-5">
                    <Button asChild variant="secondary" className="w-full">
                      <Link href="/">Back to the home page</Link>
                    </Button>
                  </div>
                </div>
              ) : (
                <>
                  <h2 className="text-xl text-ink">Join the waitlist</h2>
                  <p className="mt-1 text-[13px] text-muted">
                    One email. A confirmation link. That is the whole step.
                  </p>

                  <form onSubmit={handleSubmit(onSubmit)} className="mt-6 space-y-4" noValidate>
                    <div className="space-y-1.5">
                      <label htmlFor="email" className="block text-[13px] font-medium text-ink">
                        Email
                      </label>
                      <Input
                        id="email"
                        {...register("email")}
                        type="email"
                        autoComplete="email"
                        placeholder="you@example.com"
                        error={errors.email?.message}
                      />
                    </div>

                    <div className="space-y-1.5">
                      <label htmlFor="displayName" className="block text-[13px] font-medium text-ink">
                        Name <span className="font-normal text-muted">(optional)</span>
                      </label>
                      <Input
                        id="displayName"
                        {...register("displayName")}
                        type="text"
                        autoComplete="name"
                        placeholder="What we should call you"
                        error={errors.displayName?.message}
                      />
                    </div>

                    {refCode && (
                      <p className="text-[12px] text-muted">
                        Referred by <span className="num text-ink-2">{refCode}</span>
                      </p>
                    )}

                    {error && (
                      <p className="text-[13px] text-rust" aria-live="polite">
                        {error}
                      </p>
                    )}

                    <Button type="submit" className="w-full" size="lg" loading={isLoading}>
                      Join the waitlist
                      <ArrowRight className="h-4 w-4 transition-transform duration-200 group-hover/btn:translate-x-0.5" strokeWidth={1.8} />
                    </Button>
                  </form>

                  <p className="mt-4 text-[12px] leading-relaxed text-muted">
                    No spam. Waitlist updates only.
                  </p>
                </>
              )}
            </div>
          </div>
        </section>
      </main>

      <SiteFooter />
    </div>
  )
}

export default function WaitlistPage() {
  return (
    <Suspense
      fallback={
        <div className="flex min-h-screen items-center justify-center bg-paper text-muted">
          Loading
        </div>
      }
    >
      <WaitlistContent />
    </Suspense>
  )
}
