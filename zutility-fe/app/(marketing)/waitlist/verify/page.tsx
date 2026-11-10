"use client"

import * as React from "react"
import Link from "next/link"
import { useSearchParams } from "next/navigation"
import { Suspense } from "react"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import * as z from "zod"
import { ArrowRight, Check, Loader2, MailCheck } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { AuthShell } from "@/components/auth/auth-shell"
import { apiPost } from "@/lib/api"

const resendSchema = z.object({
  email: z.string().email("Invalid email address"),
})

type ResendFormValues = z.infer<typeof resendSchema>

interface VerifyResult {
  email: string
  position: number
  referral_code: string
}

function WaitlistVerifyContent() {
  const searchParams = useSearchParams()
  const tokenParam = searchParams.get("token")
  const [isVerifying, setIsVerifying] = React.useState(Boolean(tokenParam))
  const [verified, setVerified] = React.useState<VerifyResult | null>(null)
  const [verifyFailed, setVerifyFailed] = React.useState(false)
  const [cooldown, setCooldown] = React.useState(60)
  const [isResending, setIsResending] = React.useState(false)
  const [resendError, setResendError] = React.useState<string | null>(null)
  const [resent, setResent] = React.useState(false)

  React.useEffect(() => {
    if (cooldown <= 0) return
    const timer = setTimeout(() => setCooldown(cooldown - 1), 1000)
    return () => clearTimeout(timer)
  }, [cooldown])

  React.useEffect(() => {
    if (!tokenParam) return
    apiPost<VerifyResult>("/api/v1/waitlist/verify", { token: tokenParam })
      .then((result) => {
        setVerified(result)
        setVerifyFailed(false)
      })
      .catch(() => {
        setVerifyFailed(true)
      })
      .finally(() => setIsVerifying(false))
  }, [tokenParam])

  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<ResendFormValues>({
    resolver: zodResolver(resendSchema),
  })

  const handleResend = async (data: ResendFormValues) => {
    if (cooldown > 0) return
    setIsResending(true)
    setResendError(null)
    setResent(false)
    try {
      await apiPost("/api/v1/waitlist/resend", { email: data.email })
      setResent(true)
      setCooldown(60)
    } catch {
      setResendError("The email did not go out. Try again in a moment.")
    } finally {
      setIsResending(false)
    }
  }

  const shareUrl = typeof window !== "undefined" && verified
    ? `${window.location.origin}/waitlist?ref=${verified.referral_code}`
    : ""

  const status = verified
    ? "Your place is held."
    : isVerifying
      ? "Checking the link."
      : verifyFailed
        ? "This link is invalid or has expired."
        : "Waiting for the link to be opened."

  return (
    <AuthShell
      title={verified ? "Email verified" : "Verify your email"}
      subtitle={
        verified
          ? `You are #${verified.position} in line for early access.`
          : "Open the link in the waitlist email to hold your place."
      }
    >
      <div className="flex items-center gap-3 rounded-lg border border-line bg-paper px-4 py-3">
        <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-md bg-gold/20 text-gold-deep">
          {isVerifying && !verified ? (
            <Loader2 className="h-4 w-4 animate-spin" strokeWidth={1.6} />
          ) : (
            <MailCheck className="h-4 w-4" strokeWidth={1.6} />
          )}
        </span>
        <p className="text-[13px] text-ink-2" aria-live="polite">
          {status}
        </p>
      </div>

      {verified ? (
        <div className="mt-5">
          <dl className="rounded-lg border border-line bg-paper">
            <div className="flex items-baseline justify-between border-b border-line px-4 py-3">
              <dt className="text-[13px] text-muted">Position</dt>
              <dd className="num text-[15px] text-ink">#{verified.position}</dd>
            </div>
            <div className="flex items-baseline justify-between border-b border-line px-4 py-3">
              <dt className="text-[13px] text-muted">Referral code</dt>
              <dd className="num text-[13px] text-ink">{verified.referral_code}</dd>
            </div>
            <div className="px-4 py-3">
              <dt className="text-[13px] text-muted">Referral link</dt>
              <dd className="num mt-1 break-all text-[12px] text-ink-2">{shareUrl}</dd>
            </div>
          </dl>

          <Button asChild className="mt-4 w-full" size="lg">
            <Link href="/signup">
              Create an account
              <ArrowRight className="h-4 w-4 transition-transform duration-200 group-hover/btn:translate-x-0.5" strokeWidth={1.8} />
            </Link>
          </Button>
          <p className="mt-3 text-center text-[12px] text-muted">
            You can also{" "}
            <Link
              href="/waitlist"
              className="text-gold-deep u-link"
            >
              return to the waitlist page
            </Link>
            .
          </p>
        </div>
      ) : isVerifying ? null : (
        <div className="mt-5 border-t border-line pt-5">
          <p className="text-[13px] text-muted">
            {verifyFailed ? "Request a new link." : "Nothing arrived?"}
          </p>
          <form onSubmit={handleSubmit(handleResend)} className="mt-3 space-y-3" noValidate>
            <Input
              {...register("email")}
              type="email"
              autoComplete="email"
              placeholder="you@example.com"
              aria-label="Email address"
              error={errors.email?.message}
            />
            <Button
              variant="secondary"
              type="submit"
              disabled={cooldown > 0 || isResending}
              loading={isResending}
              className="w-full"
            >
              {cooldown > 0 ? `Resend in ${cooldown}s` : "Resend the email"}
            </Button>
          </form>
          {resent && (
            <p className="mt-3 flex items-center gap-1.5 text-[12px] text-green">
              <Check className="h-4 w-4" strokeWidth={1.8} />
              A new link is on the way.
            </p>
          )}
          {resendError && (
            <p className="mt-3 text-[12px] text-rust" aria-live="polite">
              {resendError}
            </p>
          )}
        </div>
      )}
    </AuthShell>
  )
}

export default function WaitlistVerifyPage() {
  return (
    <Suspense
      fallback={
        <div className="flex min-h-screen items-center justify-center bg-paper text-muted">
          Loading
        </div>
      }
    >
      <WaitlistVerifyContent />
    </Suspense>
  )
}
