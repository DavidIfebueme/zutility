"use client"

import * as React from "react"
import Link from "next/link"
import { useRouter, useSearchParams } from "next/navigation"
import { Suspense } from "react"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import * as z from "zod"
import { motion } from "motion/react"
import LockIcon from "@/components/icons/lock-icon"
import ArrowNarrowRightIcon from "@/components/icons/arrow-narrow-right-icon"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { apiPostRaw } from "@/lib/api"
import { toast } from "sonner"
import dynamic from "next/dynamic"

const ZecCoinScene = dynamic(() => import("@/components/3d/ZecCoin").then(mod => mod.ZecCoinScene), { ssr: false })
const Canvas = dynamic(() => import("@react-three/fiber").then(mod => mod.Canvas), { ssr: false })

const resetSchema = z.object({
  password: z.string().min(8, "Password must be at least 8 characters"),
  confirmPassword: z.string()
}).refine((data) => data.password === data.confirmPassword, {
  message: "Passwords don't match",
  path: ["confirmPassword"],
})

type ResetFormValues = z.infer<typeof resetSchema>

function ResetPasswordContent() {
  const router = useRouter()
  const searchParams = useSearchParams()
  const token = searchParams.get("token")
  const [isLoading, setIsLoading] = React.useState(false)
  const [success, setSuccess] = React.useState(false)

  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<ResetFormValues>({
    resolver: zodResolver(resetSchema),
  })

  const onSubmit = async (data: ResetFormValues) => {
    if (!token) {
      toast.error("Invalid or missing reset token.")
      return
    }
    setIsLoading(true)
    try {
      const res = await apiPostRaw("/api/v1/auth/reset-password", {
        token,
        password: data.password,
      })
      if (res.ok) {
        setSuccess(true)
        toast.success("Password reset successfully!")
        setTimeout(() => router.push("/login"), 2000)
      } else {
        const result = await res.json().catch(() => ({ error: "Reset failed" }))
        toast.error(result.error || "Failed to reset password.")
      }
    } catch {
      toast.error("Failed to reset password. Please try again.")
    } finally {
      setIsLoading(false)
    }
  }

  if (!token) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-bg-void text-text-primary p-4">
        <div className="text-center max-w-sm">
          <h2 className="font-dela text-2xl mb-4">Invalid Link</h2>
          <p className="text-text-secondary mb-6">
            This password reset link is invalid or has expired.
          </p>
          <Link href="/forgot-password" className="text-accent-zec hover:underline">
            Request a new reset link
          </Link>
        </div>
      </div>
    )
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
            Almost there.
          </h1>
          <p className="text-text-secondary text-lg">
            Set your new password and regain access to your account.
          </p>
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
            <h2 className="font-dela text-3xl mb-2">Reset Password</h2>
            <p className="text-text-secondary mb-8">
              Enter your new password below.
            </p>

            {success ? (
              <div className="rounded-lg border border-border-subtle bg-bg-elevated p-6 text-center">
                <h3 className="font-semibold mb-2">Password reset!</h3>
                <p className="text-sm text-text-secondary">
                  Redirecting to login...
                </p>
              </div>
            ) : (
              <form onSubmit={handleSubmit(onSubmit)} className="space-y-5">
                <div className="space-y-1">
                  <Input
                    {...register("password")}
                    type="password"
                    placeholder="New password"
                    leftIcon={<LockIcon size={20} />}
                    error={errors.password?.message}
                  />
                </div>

                <div className="space-y-1">
                  <Input
                    {...register("confirmPassword")}
                    type="password"
                    placeholder="Confirm new password"
                    leftIcon={<LockIcon size={20} />}
                    error={errors.confirmPassword?.message}
                  />
                </div>

                <Button type="submit" className="w-full h-12 text-base mt-4" loading={isLoading}>
                  Reset Password <ArrowNarrowRightIcon size={16} className="ml-2" />
                </Button>
              </form>
            )}
          </motion.div>
        </div>
      </div>
    </div>
  )
}

export default function ResetPasswordPage() {
  return (
    <Suspense fallback={<div className="flex min-h-screen items-center justify-center bg-bg-void text-text-muted">Loading...</div>}>
      <ResetPasswordContent />
    </Suspense>
  )
}
