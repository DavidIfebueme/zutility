"use client"

import * as React from "react"
import Link from "next/link"
import { useRouter, useSearchParams } from "next/navigation"
import { Suspense } from "react"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import * as z from "zod"
import { Eye, EyeOff } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { AuthShell } from "@/components/auth/auth-shell"
import { apiPostRaw } from "@/lib/api"
import { toast } from "sonner"

const resetSchema = z.object({
  password: z.string().min(8, "Use at least 8 characters"),
  confirmPassword: z.string(),
}).refine((data) => data.password === data.confirmPassword, {
  message: "Passwords do not match",
  path: ["confirmPassword"],
})

type ResetFormValues = z.infer<typeof resetSchema>

function ResetPasswordContent() {
  const router = useRouter()
  const searchParams = useSearchParams()
  const token = searchParams.get("token")
  const [isLoading, setIsLoading] = React.useState(false)
  const [success, setSuccess] = React.useState(false)
  const [showPassword, setShowPassword] = React.useState(false)

  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<ResetFormValues>({
    resolver: zodResolver(resetSchema),
  })

  const onSubmit = async (data: ResetFormValues) => {
    if (!token) {
      toast.error("The reset token is missing")
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
        toast.success("Password changed")
        setTimeout(() => router.push("/login"), 2000)
      } else {
        const result = await res.json().catch(() => ({ error: "Reset failed" }))
        toast.error(result.error || "Could not reset the password")
      }
    } catch {
      toast.error("Could not reach the server")
    } finally {
      setIsLoading(false)
    }
  }

  if (!token) {
    return (
      <AuthShell
        title="Link expired"
        subtitle="This reset link is invalid or has already been used."
      >
        <Button asChild className="w-full" size="lg">
          <Link href="/forgot-password">Request a new link</Link>
        </Button>
      </AuthShell>
    )
  }

  return (
    <AuthShell
      title={success ? "Password changed" : "Choose a new password"}
      subtitle={
        success
          ? "Opening the sign in page."
          : "Use at least eight characters. A longer phrase is better than a short scramble."
      }
    >
      {success ? (
        <div className="rounded-lg border border-line bg-paper px-4 py-3 text-[13px] text-ink-2">
          The new password is active.
        </div>
      ) : (
        <form onSubmit={handleSubmit(onSubmit)} className="space-y-4" noValidate>
          <div className="space-y-1.5">
            <label htmlFor="password" className="block text-[13px] font-medium text-ink">
              New password
            </label>
            <Input
              id="password"
              {...register("password")}
              type={showPassword ? "text" : "password"}
              autoComplete="new-password"
              placeholder="At least 8 characters"
              error={errors.password?.message}
              rightElement={
                <button
                  type="button"
                  onClick={() => setShowPassword((value) => !value)}
                  className="flex h-8 w-8 items-center justify-center rounded-md text-muted transition-colors hover:text-ink"
                  aria-label={showPassword ? "Hide password" : "Show password"}
                >
                  {showPassword ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                </button>
              }
            />
          </div>

          <div className="space-y-1.5">
            <label htmlFor="confirmPassword" className="block text-[13px] font-medium text-ink">
              Confirm password
            </label>
            <Input
              id="confirmPassword"
              {...register("confirmPassword")}
              type={showPassword ? "text" : "password"}
              autoComplete="new-password"
              placeholder="Repeat the password"
              error={errors.confirmPassword?.message}
            />
          </div>

          <Button type="submit" className="mt-2 w-full" size="lg" loading={isLoading}>
            Set the password
          </Button>
        </form>
      )}
    </AuthShell>
  )
}

export default function ResetPasswordPage() {
  return (
    <Suspense
      fallback={
        <div className="flex min-h-screen items-center justify-center bg-paper text-muted">
          Loading
        </div>
      }
    >
      <ResetPasswordContent />
    </Suspense>
  )
}
