"use client"

import * as React from "react"
import Link from "next/link"
import { useRouter } from "next/navigation"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import * as z from "zod"
import { Eye, EyeOff } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { AuthFooterLink, AuthShell } from "@/components/auth/auth-shell"
import { useAuthStore } from "@/store/auth"
import { apiPostRaw } from "@/lib/api"
import { toast } from "sonner"

const loginSchema = z.object({
  email: z.string().email("Enter a valid email address"),
  password: z.string().min(1, "Enter your password"),
})

type LoginFormValues = z.infer<typeof loginSchema>

export default function LoginPage() {
  const router = useRouter()
  const { setUser } = useAuthStore()
  const [isLoading, setIsLoading] = React.useState(false)
  const [showPassword, setShowPassword] = React.useState(false)

  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<LoginFormValues>({
    resolver: zodResolver(loginSchema),
  })

  const onSubmit = async (data: LoginFormValues) => {
    setIsLoading(true)
    try {
      const res = await apiPostRaw("/api/v1/auth/login", {
        email: data.email,
        password: data.password,
      })
      if (res.ok) {
        const result = await res.json()
        setUser(result)
        toast.success("Signed in")
        router.push("/dashboard")
      } else if (res.status === 403) {
        const result = await res.json()
        if (result.error === "email_not_verified") {
          toast.error("Verify your email before signing in")
          router.push(`/verify?email=${encodeURIComponent(data.email)}`)
        } else {
          toast.error(result.error || "Access denied")
        }
      } else {
        const result = await res.json().catch(() => ({ error: "Login failed" }))
        toast.error(result.error || "Email or password is wrong")
      }
    } catch {
      toast.error("Could not reach the server")
    } finally {
      setIsLoading(false)
    }
  }

  return (
    <AuthShell
      title="Sign in"
      subtitle="Your orders, addresses, and receipts in one place."
      footer={<AuthFooterLink prompt="No account yet?" href="/signup" label="Create one" />}
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
          <div className="flex items-baseline justify-between">
            <label htmlFor="password" className="block text-[13px] font-medium text-ink">
              Password
            </label>
            <Link
              href="/forgot-password"
              className="text-[12px] text-muted transition-colors hover:text-ink"
            >
              Forgot password
            </Link>
          </div>
          <Input
            id="password"
            {...register("password")}
            type={showPassword ? "text" : "password"}
            autoComplete="current-password"
            placeholder="Your password"
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

        <Button type="submit" className="mt-2 w-full" size="lg" loading={isLoading}>
          Sign in
        </Button>
      </form>
    </AuthShell>
  )
}
