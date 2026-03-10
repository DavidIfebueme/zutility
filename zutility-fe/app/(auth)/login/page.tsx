"use client"

import * as React from "react"
import Link from "next/link"
import { useRouter } from "next/navigation"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import * as z from "zod"
import { motion } from "motion/react"
import { Mail, Lock, ArrowRight } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { useAuthStore } from "@/store/auth"
import { toast } from "sonner"
import dynamic from "next/dynamic"

const ZecCoinScene = dynamic(() => import("@/components/3d/ZecCoin").then(mod => mod.ZecCoinScene), { ssr: false })
const Canvas = dynamic(() => import("@react-three/fiber").then(mod => mod.Canvas), { ssr: false })

const loginSchema = z.object({
  email: z.string().email("Invalid email address"),
  password: z.string().min(1, "Password is required"),
})

type LoginFormValues = z.infer<typeof loginSchema>

export default function LoginPage() {
  const router = useRouter()
  const { login } = useAuthStore()
  const [isLoading, setIsLoading] = React.useState(false)

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
      // Mock API call
      // const res = await fetch(`${process.env.NEXT_PUBLIC_API_URL}/api/v1/auth/login`, { ... })
      await new Promise(resolve => setTimeout(resolve, 1500))
      
      if (data.email === "test@example.com" && data.password === "password") {
        login({ email: data.email, displayName: "Test User" }, "mock-jwt-token")
        toast.success("Welcome back!")
        router.push("/dashboard")
      } else {
        toast.error("Invalid email or password. Try test@example.com / password")
      }
    } catch (error) {
      toast.error("An error occurred during login.")
    } finally {
      setIsLoading(false)
    }
  }

  return (
    <div className="flex min-h-screen bg-bg-void text-text-primary">
      {/* Left side - 3D & Branding (Hidden on mobile) */}
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
            Welcome back to the private economy.
          </h1>
          <p className="text-text-secondary text-lg">
            Access your dashboard to track orders, manage settings, and pay utilities with zero KYC.
          </p>
        </div>
      </div>

      {/* Right side - Form */}
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
            <h2 className="font-dela text-3xl mb-2">Log In</h2>
            <p className="text-text-secondary mb-8">
              Don&apos;t have an account?{" "}
              <Link href="/signup" className="text-accent-zec hover:underline">
                Sign up
              </Link>
            </p>

            <form onSubmit={handleSubmit(onSubmit)} className="space-y-5">
              <div className="space-y-1">
                <Input
                  {...register("email")}
                  type="email"
                  placeholder="Email address"
                  leftIcon={<Mail className="h-5 w-5" />}
                  error={errors.email?.message}
                />
              </div>
              
              <div className="space-y-1">
                <Input
                  {...register("password")}
                  type="password"
                  placeholder="Password"
                  leftIcon={<Lock className="h-5 w-5" />}
                  error={errors.password?.message}
                />
                <div className="flex justify-end pt-1">
                  <Link href="#" className="text-xs text-text-muted hover:text-text-primary transition-colors">
                    Forgot password?
                  </Link>
                </div>
              </div>

              <Button type="submit" className="w-full h-12 text-base mt-4" loading={isLoading}>
                Log In <ArrowRight className="ml-2 h-4 w-4" />
              </Button>
            </form>
          </motion.div>
        </div>
      </div>
    </div>
  )
}
