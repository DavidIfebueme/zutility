"use client"

import * as React from "react"
import Link from "next/link"
import { useRouter, useSearchParams } from "next/navigation"
import { Suspense } from "react"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import * as z from "zod"
import { motion } from "motion/react"
import { Mail, ArrowRight, RefreshCw, CheckCircle2 } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card"
import { apiPost } from "@/lib/api"
import { toast } from "sonner"

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
  const router = useRouter()
  const searchParams = useSearchParams()
  const tokenParam = searchParams.get("token")
  const [isVerifying, setIsVerifying] = React.useState(false)
  const [verified, setVerified] = React.useState<VerifyResult | null>(null)
  const [cooldown, setCooldown] = React.useState(60)
  const [isResending, setIsResending] = React.useState(false)

  React.useEffect(() => {
    if (cooldown > 0) {
      const timer = setTimeout(() => setCooldown(cooldown - 1), 1000)
      return () => clearTimeout(timer)
    }
  }, [cooldown])

  React.useEffect(() => {
    if (!tokenParam) return
    setIsVerifying(true)
    apiPost<VerifyResult>("/api/v1/waitlist/verify", { token: tokenParam })
      .then((result) => {
        setVerified(result)
        toast.success("Email verified! You're on the list.")
      })
      .catch(() => {
        toast.error("Invalid or expired verification link.")
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
    try {
      await apiPost("/api/v1/waitlist/resend", { email: data.email })
      toast.success("Verification email resent!")
      setCooldown(60)
    } catch {
      toast.error("Failed to resend. Please try again.")
    } finally {
      setIsResending(false)
    }
  }

  const shareUrl = typeof window !== "undefined" && verified
    ? `${window.location.origin}/waitlist?ref=${verified.referral_code}`
    : ""

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
        <Card className="border-border-subtle shadow-2xl">
          <CardHeader className="text-center pb-2">
            <div className="mx-auto mb-6 flex h-16 w-16 items-center justify-center rounded-full bg-accent-zec/10 text-accent-zec">
              {verified ? (
                <CheckCircle2 className="h-8 w-8" />
              ) : (
                <Mail className="h-8 w-8" />
              )}
            </div>
            <CardTitle className="text-2xl font-dela">
              {verified ? "You're verified!" : "Verify your email"}
            </CardTitle>
            <CardDescription className="text-base mt-2">
              {verified
                ? `You're #${verified.position} in line for early access.`
                : isVerifying
                  ? "Verifying your email..."
                  : "Confirm your email to secure your spot on the waitlist."}
            </CardDescription>
          </CardHeader>
          <CardContent className="flex flex-col items-center gap-6 pt-6">
            {verified ? (
              <div className="w-full space-y-4">
                <div className="rounded-lg bg-bg-elevated border border-border-subtle p-4 text-center">
                  <p className="text-sm text-text-secondary mb-1">Your position</p>
                  <p className="text-3xl font-dela text-accent-zec">#{verified.position}</p>
                </div>
                <div className="rounded-lg bg-bg-elevated border border-border-subtle p-4">
                  <p className="text-sm text-text-secondary mb-2">Share your referral link</p>
                  <code className="text-xs text-accent-zec break-all">{shareUrl}</code>
                  <p className="text-xs text-text-muted mt-2">
                    Code: <span className="font-mono text-text-primary">{verified.referral_code}</span>
                  </p>
                </div>
                <Link href="/signup">
                  <Button variant="primary" className="w-full mt-2">
                    Create Account <ArrowRight className="ml-2 h-4 w-4" />
                  </Button>
                </Link>
              </div>
            ) : isVerifying ? (
              <div className="flex items-center gap-2 text-sm text-text-muted">
                <RefreshCw className="h-4 w-4 animate-spin" />
                Verifying your email...
              </div>
            ) : (
              <div className="w-full border-t border-border-subtle pt-6 text-center">
                <p className="text-sm text-text-secondary mb-4">
                  Need a new verification email?
                </p>
                <form onSubmit={handleSubmit(handleResend)} className="space-y-3">
                  <Input
                    {...register("email")}
                    type="email"
                    placeholder="Your email address"
                    leftIcon={<Mail className="h-5 w-5" />}
                    error={errors.email?.message}
                  />
                  <Button
                    variant="secondary"
                    type="submit"
                    disabled={cooldown > 0 || isResending}
                    loading={isResending}
                    className="w-full"
                  >
                    {cooldown > 0 ? `Resend available in ${cooldown}s` : "Resend Email"}
                  </Button>
                </form>
              </div>
            )}
          </CardContent>
        </Card>
      </motion.div>
    </div>
  )
}

export default function WaitlistVerifyPage() {
  return (
    <Suspense fallback={<div className="flex min-h-screen items-center justify-center bg-bg-void text-text-muted">Loading...</div>}>
      <WaitlistVerifyContent />
    </Suspense>
  )
}
