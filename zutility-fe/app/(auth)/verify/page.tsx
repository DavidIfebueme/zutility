"use client"

import * as React from "react"
import { useRouter, useSearchParams } from "next/navigation"
import { Suspense } from "react"
import { Loader2, MailCheck } from "lucide-react"
import { Button } from "@/components/ui/button"
import { AuthShell } from "@/components/auth/auth-shell"
import { useAuthStore } from "@/store/auth"
import { apiGet, apiPost, apiPostRaw } from "@/lib/api"
import { toast } from "sonner"

function VerifyContent() {
  const router = useRouter()
  const searchParams = useSearchParams()
  const { setUser } = useAuthStore()
  const email = searchParams.get("email") || ""
  const tokenParam = searchParams.get("token")
  const [cooldown, setCooldown] = React.useState(60)
  const [isResending, setIsResending] = React.useState(false)
  const [isVerifying, setIsVerifying] = React.useState(Boolean(tokenParam))
  const [verified, setVerified] = React.useState(false)

  React.useEffect(() => {
    if (cooldown <= 0) return
    const timer = setTimeout(() => setCooldown(cooldown - 1), 1000)
    return () => clearTimeout(timer)
  }, [cooldown])

  React.useEffect(() => {
    if (!tokenParam) return
    apiPostRaw("/api/v1/auth/verify-email", { token: tokenParam })
      .then(async (res) => {
        if (res.ok) {
          const result = await res.json()
          setUser(result)
          setVerified(true)
          toast.success("Email verified")
          setTimeout(() => router.push("/dashboard"), 1500)
        } else {
          toast.error("This link is invalid or has expired")
        }
      })
      .catch(() => toast.error("Verification failed"))
      .finally(() => setIsVerifying(false))
  }, [tokenParam, router, setUser])

  React.useEffect(() => {
    if (!email || tokenParam) return
    let active = true
    const poll = setInterval(async () => {
      const result = await apiGet<{ email_verified: boolean }>("/api/v1/auth/me").catch(() => null)
      if (result?.email_verified && active) {
        clearInterval(poll)
        setVerified(true)
        toast.success("Email verified")
        setTimeout(() => router.push("/dashboard"), 1500)
      }
    }, 5000)
    return () => {
      active = false
      clearInterval(poll)
    }
  }, [email, tokenParam, router])

  const handleResend = async () => {
    if (cooldown > 0 || !email) return
    setIsResending(true)
    try {
      await apiPost("/api/v1/auth/resend-verification", { email })
      toast.success("Verification email sent")
      setCooldown(60)
    } catch {
      toast.error("Could not resend the email")
    } finally {
      setIsResending(false)
    }
  }

  return (
    <AuthShell
      title={verified ? "Email verified" : "Check your email"}
      subtitle={
        verified
          ? "Opening your dashboard."
          : `We sent a verification link to ${email || "your email address"}. Open it to finish setting up the account.`
      }
    >
      <div className="flex items-center gap-3 rounded-lg border border-line bg-paper px-4 py-3">
        <span className="flex h-9 w-9 items-center justify-center rounded-md bg-gold/20 text-gold-deep">
          {verified ? (
            <MailCheck className="h-4 w-4" />
          ) : (
            <Loader2 className="h-4 w-4 animate-spin" />
          )}
        </span>
        <p className="text-[13px] text-ink-2">
          {verified
            ? "Redirecting now."
            : isVerifying
              ? "Checking the link."
              : "Waiting for the link to be opened."}
        </p>
      </div>

      {!verified && (
        <div className="mt-5 border-t border-line pt-5">
          <p className="text-[13px] text-muted">Nothing arrived?</p>
          <Button
            variant="secondary"
            onClick={handleResend}
            disabled={cooldown > 0 || isResending || !email}
            loading={isResending}
            className="mt-2 w-full"
          >
            {cooldown > 0 ? `Resend in ${cooldown}s` : "Resend the email"}
          </Button>
        </div>
      )}
    </AuthShell>
  )
}

export default function VerifyPage() {
  return (
    <Suspense
      fallback={
        <div className="flex min-h-screen items-center justify-center bg-paper text-muted">
          Loading
        </div>
      }
    >
      <VerifyContent />
    </Suspense>
  )
}
