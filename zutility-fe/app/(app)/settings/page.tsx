"use client"

import * as React from "react"
import { useRouter } from "next/navigation"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import * as z from "zod"
import { motion } from "motion/react"
import {
  User,
  Mail,
  Lock,
  Shield,
  Globe,
  Trash2,
  AlertTriangle,
  CheckCircle2,
  Loader2,
  ChevronRight,
} from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
  DialogTrigger,
} from "@/components/ui/modal"
import { useAuthStore } from "@/store/auth"
import { apiPost } from "@/lib/api"
import { useCurrency } from "@/lib/hooks/useCurrency"
import { CURRENCIES, type CurrencyCode } from "@/lib/currency"
import { toast } from "sonner"

const profileSchema = z.object({
  display_name: z
    .string()
    .min(1, "Display name cannot be empty")
    .max(100, "Display name must be 100 characters or less"),
})

type ProfileFormValues = z.infer<typeof profileSchema>

const changePasswordSchema = z
  .object({
    current_password: z.string().min(1, "Current password is required"),
    new_password: z.string().min(8, "New password must be at least 8 characters"),
    confirm_password: z.string().min(1, "Please confirm your new password"),
  })
  .refine((data) => data.new_password === data.confirm_password, {
    message: "Passwords do not match",
    path: ["confirm_password"],
  })

type ChangePasswordFormValues = z.infer<typeof changePasswordSchema>

const deleteAccountSchema = z.object({
  password: z.string().min(1, "Password is required"),
})

type DeleteAccountFormValues = z.infer<typeof deleteAccountSchema>

export default function SettingsPage() {
  const router = useRouter()
  const { user, setUser, logout, preferredCurrency, setPreferredCurrency } = useAuthStore()
  const activeCurrency = useCurrency()

  const [profileLoading, setProfileLoading] = React.useState(false)
  const [passwordLoading, setPasswordLoading] = React.useState(false)
  const [deleteLoading, setDeleteLoading] = React.useState(false)
  const [deleteDialogOpen, setDeleteDialogOpen] = React.useState(false)
  const [resendLoading, setResendLoading] = React.useState(false)

  const profileForm = useForm<ProfileFormValues>({
    resolver: zodResolver(profileSchema),
    defaultValues: {
      display_name: user?.display_name || "",
    },
  })

  const passwordForm = useForm<ChangePasswordFormValues>({
    resolver: zodResolver(changePasswordSchema),
    defaultValues: {
      current_password: "",
      new_password: "",
      confirm_password: "",
    },
  })

  const deleteForm = useForm<DeleteAccountFormValues>({
    resolver: zodResolver(deleteAccountSchema),
    defaultValues: { password: "" },
  })

  React.useEffect(() => {
    if (user) {
      profileForm.reset({ display_name: user.display_name || "" })
    }
  }, [user?.display_name])

  const onProfileSubmit = async (data: ProfileFormValues) => {
    setProfileLoading(true)
    try {
      const updated = await apiPost<{ id: string; email: string; display_name: string | null; email_verified: boolean }>("/api/v1/auth/profile", {
        display_name: data.display_name.trim(),
      })
      setUser(updated)
      toast.success("Profile updated")
    } catch {
      toast.error("Failed to update profile")
    } finally {
      setProfileLoading(false)
    }
  }

  const onPasswordSubmit = async (data: ChangePasswordFormValues) => {
    setPasswordLoading(true)
    try {
      await apiPost("/api/v1/auth/change-password", {
        current_password: data.current_password,
        new_password: data.new_password,
      })
      toast.success("Password changed. Please log in again.")
      logout()
      router.push("/login")
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : ""
      if (msg.includes("incorrect")) {
        toast.error("Current password is incorrect")
        passwordForm.setError("current_password", { message: "Incorrect password" })
      } else {
        toast.error("Failed to change password")
      }
    } finally {
      setPasswordLoading(false)
    }
  }

  const onDeleteSubmit = async (data: DeleteAccountFormValues) => {
    setDeleteLoading(true)
    try {
      await apiPost("/api/v1/auth/delete-account", {
        password: data.password,
      })
      toast.success("Account deleted")
      setDeleteDialogOpen(false)
      logout()
      router.push("/")
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : ""
      if (msg.includes("incorrect")) {
        toast.error("Password is incorrect")
        deleteForm.setError("password", { message: "Incorrect password" })
      } else {
        toast.error("Failed to delete account")
      }
    } finally {
      setDeleteLoading(false)
    }
  }

  const onResendVerification = async () => {
    if (!user?.email) return
    setResendLoading(true)
    try {
      await apiPost("/api/v1/auth/resend-verification", { email: user.email })
      toast.success("Verification email sent")
    } catch {
      toast.error("Failed to send verification email")
    } finally {
      setResendLoading(false)
    }
  }

  if (!user) return null

  return (
    <div className="space-y-8 max-w-2xl">
      <div>
        <h1 className="font-dela text-3xl tracking-tight">Settings</h1>
        <p className="text-text-secondary mt-2">Manage your account and preferences.</p>
      </div>

      <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.3 }}>
        <Card>
          <CardHeader>
            <div className="flex items-center gap-3">
              <div className="h-10 w-10 rounded-full bg-accent-zec/10 text-accent-zec flex items-center justify-center">
                <User className="h-5 w-5" />
              </div>
              <div>
                <CardTitle className="text-xl">Profile</CardTitle>
                <CardDescription>Your public display name and account info</CardDescription>
              </div>
            </div>
          </CardHeader>
          <CardContent>
            <form onSubmit={profileForm.handleSubmit(onProfileSubmit)} className="space-y-5">
              <div className="space-y-2">
                <label className="text-sm font-medium text-text-secondary">Email</label>
                <div className="flex items-center gap-3">
                  <Input value={user.email} disabled leftIcon={<Mail className="h-5 w-5" />} />
                  {user.email_verified ? (
                    <Badge variant="success" className="shrink-0">
                      <CheckCircle2 className="h-3 w-3 mr-1" />
                      Verified
                    </Badge>
                  ) : (
                    <Badge variant="error" className="shrink-0">Unverified</Badge>
                  )}
                </div>
                {!user.email_verified && (
                  <button
                    type="button"
                    onClick={onResendVerification}
                    disabled={resendLoading}
                    className="text-xs text-accent-zec hover:underline disabled:opacity-50 mt-1"
                  >
                    {resendLoading ? "Sending..." : "Resend verification email"}
                  </button>
                )}
              </div>

              <div className="space-y-2">
                <label className="text-sm font-medium text-text-secondary">Display Name</label>
                <Input
                  {...profileForm.register("display_name")}
                  placeholder="Enter your display name"
                  leftIcon={<User className="h-5 w-5" />}
                  error={profileForm.formState.errors.display_name?.message}
                />
              </div>

              <div className="flex justify-end">
                <Button type="submit" loading={profileLoading}>
                  Save Changes
                </Button>
              </div>
            </form>
          </CardContent>
        </Card>
      </motion.div>

      <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.3, delay: 0.1 }}>
        <Card>
          <CardHeader>
            <div className="flex items-center gap-3">
              <div className="h-10 w-10 rounded-full bg-accent-zec/10 text-accent-zec flex items-center justify-center">
                <Lock className="h-5 w-5" />
              </div>
              <div>
                <CardTitle className="text-xl">Security</CardTitle>
                <CardDescription>Change your password</CardDescription>
              </div>
            </div>
          </CardHeader>
          <CardContent>
            <form onSubmit={passwordForm.handleSubmit(onPasswordSubmit)} className="space-y-5">
              <div className="space-y-2">
                <label className="text-sm font-medium text-text-secondary">Current Password</label>
                <Input
                  type="password"
                  {...passwordForm.register("current_password")}
                  placeholder="Enter current password"
                  leftIcon={<Shield className="h-5 w-5" />}
                  error={passwordForm.formState.errors.current_password?.message}
                />
              </div>

              <div className="space-y-2">
                <label className="text-sm font-medium text-text-secondary">New Password</label>
                <Input
                  type="password"
                  {...passwordForm.register("new_password")}
                  placeholder="At least 8 characters"
                  leftIcon={<Lock className="h-5 w-5" />}
                  error={passwordForm.formState.errors.new_password?.message}
                />
              </div>

              <div className="space-y-2">
                <label className="text-sm font-medium text-text-secondary">Confirm New Password</label>
                <Input
                  type="password"
                  {...passwordForm.register("confirm_password")}
                  placeholder="Repeat new password"
                  leftIcon={<Lock className="h-5 w-5" />}
                  error={passwordForm.formState.errors.confirm_password?.message}
                />
              </div>

              <div className="flex justify-end">
                <Button type="submit" loading={passwordLoading}>
                  Change Password
                </Button>
              </div>
            </form>
          </CardContent>
        </Card>
      </motion.div>

      <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.3, delay: 0.2 }}>
        <Card>
          <CardHeader>
            <div className="flex items-center gap-3">
              <div className="h-10 w-10 rounded-full bg-accent-zec/10 text-accent-zec flex items-center justify-center">
                <Globe className="h-5 w-5" />
              </div>
              <div>
                <CardTitle className="text-xl">Preferences</CardTitle>
                <CardDescription>Local currency display</CardDescription>
              </div>
            </div>
          </CardHeader>
          <CardContent>
            <div className="space-y-3">
              <label className="text-sm font-medium text-text-secondary">Display Currency</label>
              <div className="grid grid-cols-2 sm:grid-cols-3 gap-2">
                {(Object.keys(CURRENCIES) as CurrencyCode[]).map((code) => {
                  const currency = CURRENCIES[code]
                  const isSelected = activeCurrency === code
                  return (
                    <button
                      key={code}
                      type="button"
                      onClick={() => setPreferredCurrency(code)}
                      className={`flex items-center gap-2 p-3 rounded-lg border text-sm font-medium transition-all ${
                        isSelected
                          ? "border-accent-zec bg-accent-zec/10 text-accent-zec"
                          : "border-border-subtle bg-bg-elevated text-text-secondary hover:border-accent-zec/50"
                      }`}
                    >
                      <span className="text-base">{currency.symbol}</span>
                      <span>{code}</span>
                      {isSelected && <CheckCircle2 className="h-4 w-4 ml-auto" />}
                    </button>
                  )
                })}
              </div>
              <p className="text-xs text-text-muted mt-2">
                Prices will be displayed in your selected currency using live exchange rates.
              </p>
            </div>
          </CardContent>
        </Card>
      </motion.div>

      <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.3, delay: 0.3 }}>
        <Card className="border-accent-red/30">
          <CardHeader>
            <div className="flex items-center gap-3">
              <div className="h-10 w-10 rounded-full bg-accent-red/10 text-accent-red flex items-center justify-center">
                <Trash2 className="h-5 w-5" />
              </div>
              <div>
                <CardTitle className="text-xl text-accent-red">Danger Zone</CardTitle>
                <CardDescription>Irreversible account actions</CardDescription>
              </div>
            </div>
          </CardHeader>
          <CardContent>
            <div className="flex items-center justify-between p-4 rounded-lg bg-accent-red/5 border border-accent-red/20">
              <div className="flex items-center gap-3">
                <AlertTriangle className="h-5 w-5 text-accent-red shrink-0" />
                <div>
                  <p className="text-sm font-medium text-text-primary">Delete Account</p>
                  <p className="text-xs text-text-muted">
                    Permanently delete your account and all associated data. This action cannot be undone.
                  </p>
                </div>
              </div>
              <Dialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
                <DialogTrigger asChild>
                  <Button variant="danger" size="sm">
                    Delete
                  </Button>
                </DialogTrigger>
                <DialogContent>
                  <DialogHeader>
                    <DialogTitle>Delete Account</DialogTitle>
                    <DialogDescription>
                      This will permanently delete your account. You will be logged out immediately. This action cannot be undone.
                    </DialogDescription>
                  </DialogHeader>
                  <form onSubmit={deleteForm.handleSubmit(onDeleteSubmit)} className="space-y-4 mt-4">
                    <div className="space-y-2">
                      <label className="text-sm font-medium text-text-secondary">Confirm your password</label>
                      <Input
                        type="password"
                        {...deleteForm.register("password")}
                        placeholder="Enter your password"
                        leftIcon={<Lock className="h-5 w-5" />}
                        error={deleteForm.formState.errors.password?.message}
                      />
                    </div>
                    <DialogFooter>
                      <Button
                        type="button"
                        variant="ghost"
                        onClick={() => setDeleteDialogOpen(false)}
                      >
                        Cancel
                      </Button>
                      <Button type="submit" variant="danger" loading={deleteLoading}>
                        Delete Account
                      </Button>
                    </DialogFooter>
                  </form>
                </DialogContent>
              </Dialog>
            </div>
          </CardContent>
        </Card>
      </motion.div>
    </div>
  )
}
