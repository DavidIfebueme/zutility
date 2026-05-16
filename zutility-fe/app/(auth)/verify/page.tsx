"use client"

import * as React from "react"
import Link from "next/link"
import { useRouter, useSearchParams } from "next/navigation"
import { Suspense } from "react"
import { motion } from "motion/react"
import { Mail, ArrowRight, RefreshCw } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card"
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
  const [isVerifying, setIsVerifying] = React.useState(false)
  const [verified, setVerified] = React.useState(false)

  React.useEffect(() => {
    if (cooldown > 0) {
      const timer = setTimeout(() => setCooldown(cooldown - 1), 1000)
      return () => clearTimeout(timer)
    }
  }, [cooldown])

  React.useEffect(() => {
    if (!tokenParam) return
    setIsVerifying(true)
    apiPostRaw("/api/v1/auth/verify-email", { token: tokenParam })
      .then(async (res) => {
        if (res.ok) {
          const result = await res.json()
          setUser(result)
          setVerified(true)
          toast.success("Email verified successfully!")
          setTimeout(() => router.push("/dashboard"), 1500)
        } else {
          toast.error("Invalid or expired verification link.")
        }
      })
      .catch(() => toast.error("Verification failed."))
      .finally(() => setIsVerifying(false))
  }, [tokenParam, router, setUser])

  React.useEffect(() => {
    if (!email || tokenParam) return
    let isMounted = true
    const pollInterval = setInterval(async () => {
      try {
        const result = await apiGet<{ email_verified: boolean }>("/api/v1/auth/me").catch(() => null)
        if (result && result.email_verified && isMounted) {
          clearInterval(pollInterval)
          setVerified(true)
          toast.success("Email verified successfully!")
          setTimeout(() => router.push("/dashboard"), 1500)
        }
      } catch {
        // ignore
      }
    }, 5000)
    return () => { isMounted = false; clearInterval(pollInterval) }
  }, [email, tokenParam, router])

  const handleResend = async () => {
    if (cooldown > 0 || !email) return
    setIsResending(true)
    try {
      await apiPost("/api/v1/auth/resend-verification", { email })
      toast.success("Verification email resent!")
      setCooldown(60)
    } catch {
      toast.error("Failed to resend email. Please try again.")
    } finally {
      setIsResending(false)
    }
  }

  return (
    <Card className="border-border-subtle shadow-2xl">
      <CardHeader className="text-center pb-2">
        <div className="mx-auto mb-6 flex h-16 w-16 items-center justify-center rounded-full bg-accent-zec/10 text-accent-zec">
          <Mail className="h-8 w-8" />
        </div>
        <CardTitle className="text-2xl font-dela">Check your email</CardTitle>
        <CardDescription className="text-base mt-2">
          {verified
            ? "Your email has been verified!"
            : `We've sent a verification link to ${email || "your email"}. Please click the link to continue.`}
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col items-center gap-6 pt-6">
        {verified ? (
          <div className="flex items-center gap-2 text-sm text-accent-green">
            <ArrowRight className="h-4 w-4" />
            Redirecting to dashboard...
          </div>
        ) : isVerifying ? (
          <div className="flex items-center gap-2 text-sm text-text-muted">
            <RefreshCw className="h-4 w-4 animate-spin" />
            Verifying your email...
          </div>
        ) : (
          <div className="flex items-center gap-2 text-sm text-text-muted">
            <RefreshCw className="h-4 w-4 animate-spin" />
            Waiting for verification...
          </div>
        )}

        {!verified && (
          <div className="w-full border-t border-border-subtle pt-6 text-center">
            <p className="text-sm text-text-secondary mb-4">
              Didn&apos;t receive the email?
            </p>
            <Button
              variant="secondary"
              onClick={handleResend}
              disabled={cooldown > 0 || isResending || !email}
              loading={isResending}
              className="w-full"
            >
              {cooldown > 0 ? `Resend available in ${cooldown}s` : "Resend Email"}
            </Button>
          </div>
        )}
      </CardContent>
    </Card>
  )
}

export default function VerifyPage() {
  return (
    <div className="flex min-h-screen items-center justify-center bg-bg-void p-4">
      <div className="absolute top-8 left-8">
        <Link href="/" className="font-dela text-2xl tracking-tight">
          <span className="text-accent-zec">z</span>utility
        </Link>
      </div>

      <motion.div
        initial={{ opacity: 0, scale: 0.95 }}
        animate={{ opacity: 1, scale: 1 }}
        transition={{ duration: 0.4 }}
        className="w-full max-w-md"
      >
        <Suspense fallback={<div className="text-center text-text-muted">Loading...</div>}>
          <VerifyContent />
        </Suspense>
      </motion.div>
    </div>
  )
}
