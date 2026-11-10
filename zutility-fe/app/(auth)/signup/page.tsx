"use client"

import * as React from "react"
import { useRouter } from "next/navigation"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import * as z from "zod"
import { Eye, EyeOff } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { AuthFooterLink, AuthShell } from "@/components/auth/auth-shell"
import { apiPostRaw } from "@/lib/api"
import { toast } from "sonner"

const signupSchema = z.object({
  email: z.string().email("Enter a valid email address"),
  display_name: z.string().min(1, "Enter a display name"),
  password: z.string().min(8, "Use at least 8 characters"),
  confirm_password: z.string().min(1, "Confirm your password"),
}).refine((data) => data.password === data.confirm_password, {
  message: "Passwords do not match",
  path: ["confirm_password"],
})

type SignupFormValues = z.infer<typeof signupSchema>

export default function SignupPage() {
  const router = useRouter()
  const [isLoading, setIsLoading] = React.useState(false)
  const [showPassword, setShowPassword] = React.useState(false)

  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<SignupFormValues>({
    resolver: zodResolver(signupSchema),
  })

  const onSubmit = async (data: SignupFormValues) => {
    setIsLoading(true)
    try {
      const res = await apiPostRaw("/api/v1/auth/register", {
        email: data.email,
        password: data.password,
        display_name: data.display_name,
      })
      if (res.ok) {
        toast.success("Account created. Check your email for the code.")
        router.push(`/verify?email=${encodeURIComponent(data.email)}`)
      } else {
        const result = await res.json().catch(() => ({ error: "Registration failed" }))
        toast.error(result.error || "Could not create the account")
      }
    } catch {
      toast.error("Could not reach the server")
    } finally {
      setIsLoading(false)
    }
  }

  return (
    <AuthShell
      title="Create an account"
      subtitle="Keep your orders and receipts together. You can still pay without one."
      footer={<AuthFooterLink prompt="Already have an account?" href="/login" label="Sign in" />}
    >
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

        <div className="space-y-1.5">
          <label htmlFor="display_name" className="block text-[13px] font-medium text-ink">
            Display name
          </label>
          <Input
            id="display_name"
            {...register("display_name")}
            type="text"
            autoComplete="name"
            placeholder="How we address you"
            error={errors.display_name?.message}
          />
        </div>

        <div className="space-y-1.5">
          <label htmlFor="password" className="block text-[13px] font-medium text-ink">
            Password
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
          <label htmlFor="confirm_password" className="block text-[13px] font-medium text-ink">
            Confirm password
          </label>
          <Input
            id="confirm_password"
            {...register("confirm_password")}
            type={showPassword ? "text" : "password"}
            autoComplete="new-password"
            placeholder="Repeat the password"
            error={errors.confirm_password?.message}
          />
        </div>

        <Button type="submit" className="mt-2 w-full" size="lg" loading={isLoading}>
          Create account
        </Button>
      </form>
    </AuthShell>
  )
}
