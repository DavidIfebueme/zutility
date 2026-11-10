"use client"

import * as React from "react"
import Link from "next/link"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import * as z from "zod"
import { ArrowLeft, MailCheck } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { AuthFooterLink, AuthShell } from "@/components/auth/auth-shell"
import { apiPostRaw } from "@/lib/api"
import { toast } from "sonner"

const forgotSchema = z.object({
  email: z.string().email("Enter a valid email address"),
})

type ForgotFormValues = z.infer<typeof forgotSchema>

export default function ForgotPasswordPage() {
  const [isLoading, setIsLoading] = React.useState(false)
  const [sent, setSent] = React.useState(false)

  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<ForgotFormValues>({
    resolver: zodResolver(forgotSchema),
  })

  const onSubmit = async (data: ForgotFormValues) => {
    setIsLoading(true)
    try {
      await apiPostRaw("/api/v1/auth/forgot-password", { email: data.email })
      setSent(true)
      toast.success("If the account exists, a reset link is on the way")
    } catch {
      setSent(true)
    } finally {
      setIsLoading(false)
    }
  }

  return (
    <AuthShell
      title="Reset your password"
      subtitle="Enter the email on the account and we will send a reset link."
      footer={<AuthFooterLink prompt="Remembered it?" href="/login" label="Sign in" />}
    >
      {sent ? (
        <div className="rounded-lg border border-line bg-paper p-5">
          <div className="flex items-center gap-2 text-ink">
            <MailCheck className="h-4 w-4 text-green" />
            <p className="text-sm font-medium">Check your inbox</p>
          </div>
          <p className="mt-2 text-[13px] leading-relaxed text-ink-2">
            If an account exists for that address, a reset link is on the way. The link expires
            after one hour.
          </p>
        </div>
      ) : (
        <form onSubmit={handleSubmit(onSubmit)} className="space-y-4" noValidate>
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

          <Button type="submit" className="mt-2 w-full" size="lg" loading={isLoading}>
            Send reset link
          </Button>
        </form>
      )}

      <Link
        href="/login"
        className="mt-5 inline-flex items-center gap-1.5 text-[13px] text-muted transition-colors hover:text-ink"
      >
        <ArrowLeft className="h-3.5 w-3.5" />
        Back to sign in
      </Link>
    </AuthShell>
  )
}
