"use client"

import * as React from "react"
import Link from "next/link"
import { useRouter } from "next/navigation"
import { motion } from "motion/react"
import { Mail, ArrowRight, RefreshCw } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card"
import { toast } from "sonner"
import { useAuthStore } from "@/store/auth"

export default function VerifyPage() {
  const router = useRouter()
  const { login } = useAuthStore()
  const [cooldown, setCooldown] = React.useState(60)
  const [isResending, setIsResending] = React.useState(false)

  React.useEffect(() => {
    // Cooldown timer
    if (cooldown > 0) {
      const timer = setTimeout(() => setCooldown(cooldown - 1), 1000)
      return () => clearTimeout(timer)
    }
  }, [cooldown])

  React.useEffect(() => {
    // Poll for verification status
    let isMounted = true
    const pollInterval = setInterval(async () => {
      try {
        // Mock API call to check if verified
        // const res = await fetch(`${process.env.NEXT_PUBLIC_API_URL}/api/v1/auth/me`)
        // const data = await res.json()
        
        // Simulate auto-verification after 10s for demo purposes
        if (isMounted) {
          clearInterval(pollInterval)
          login({ email: "test@example.com", displayName: "Test User" }, "mock-jwt-token")
          toast.success("Email verified successfully!")
          router.push("/dashboard")
        }
      } catch (error) {
        console.error("Polling error", error)
      }
    }, 10000)

    return () => {
      isMounted = false
      clearInterval(pollInterval)
    }
  }, [router, login])

  const handleResend = async () => {
    if (cooldown > 0) return
    
    setIsResending(true)
    try {
      // Mock API call
      // await fetch(`${process.env.NEXT_PUBLIC_API_URL}/api/v1/auth/resend-verification`, { method: 'POST' })
      await new Promise(resolve => setTimeout(resolve, 1000))
      toast.success("Verification email resent!")
      setCooldown(60)
    } catch (error) {
      toast.error("Failed to resend email. Please try again.")
    } finally {
      setIsResending(false)
    }
  }

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
              <Mail className="h-8 w-8" />
            </div>
            <CardTitle className="text-2xl font-dela">Check your email</CardTitle>
            <CardDescription className="text-base mt-2">
              We&apos;ve sent a verification link to your email address. Please click the link to continue.
            </CardDescription>
          </CardHeader>
          <CardContent className="flex flex-col items-center gap-6 pt-6">
            <div className="flex items-center gap-2 text-sm text-text-muted">
              <RefreshCw className="h-4 w-4 animate-spin" />
              Waiting for verification...
            </div>

            <div className="w-full border-t border-border-subtle pt-6 text-center">
              <p className="text-sm text-text-secondary mb-4">
                Didn&apos;t receive the email?
              </p>
              <Button
                variant="secondary"
                onClick={handleResend}
                disabled={cooldown > 0 || isResending}
                loading={isResending}
                className="w-full"
              >
                {cooldown > 0 ? `Resend available in ${cooldown}s` : "Resend Email"}
              </Button>
            </div>
          </CardContent>
        </Card>
      </motion.div>
    </div>
  )
}
