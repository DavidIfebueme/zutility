"use client"

import * as React from "react"
import { useRouter } from "next/navigation"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import * as z from "zod"
import { Check, Globe, Lock, TriangleAlert, User } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Badge } from "@/components/ui/badge"
import { useAuthStore } from "@/store/auth"
import { apiPost } from "@/lib/api"
import { useCurrency } from "@/lib/hooks/useCurrency"
import { CURRENCIES, type CurrencyCode } from "@/lib/currency"
import { ThemePicker } from "@/components/theme-provider"
import { cn } from "@/lib/utils"
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

const SECTIONS = [
  { id: "profile", label: "Profile", icon: User },
  { id: "security", label: "Security", icon: Lock },
  { id: "preferences", label: "Preferences", icon: Globe },
  { id: "danger", label: "Danger zone", icon: TriangleAlert },
] as const

type SectionId = (typeof SECTIONS)[number]["id"]

const CURRENCY_CODES = Object.keys(CURRENCIES) as CurrencyCode[]

export default function SettingsPage() {
  const router = useRouter()
  const { user, setUser, logout, setPreferredCurrency } = useAuthStore()
  const activeCurrency = useCurrency()

  const [section, setSection] = React.useState<SectionId>("profile")
  const [profileLoading, setProfileLoading] = React.useState(false)
  const [passwordLoading, setPasswordLoading] = React.useState(false)
  const [deleteLoading, setDeleteLoading] = React.useState(false)
  const [deleteConfirmOpen, setDeleteConfirmOpen] = React.useState(false)
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
  }, [user, profileForm])

  const onProfileSubmit = async (data: ProfileFormValues) => {
    setProfileLoading(true)
    try {
      const updated = await apiPost<{
        id: string
        email: string
        display_name: string | null
        email_verified: boolean
      }>("/api/v1/auth/profile", {
        display_name: data.display_name.trim(),
      })
      setUser(updated)
      profileForm.reset({ display_name: updated.display_name || "" })
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
      setDeleteConfirmOpen(false)
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
    <div className="space-y-8">
      <header>
        <p className="label-xs text-muted">Account</p>
        <h1 className="mt-2 text-3xl text-ink">Settings</h1>
        <p className="mt-1.5 max-w-lg text-[14px] text-muted">
          Your name on receipts, the password you sign in with, and how the app looks.
        </p>
      </header>

      <div className="grid gap-7 lg:grid-cols-[176px_1fr] lg:gap-9">
        <nav
          aria-label="Settings sections"
          className="-mx-1 flex gap-1 overflow-x-auto px-1 pb-1 lg:mx-0 lg:flex-col lg:overflow-visible lg:px-0 lg:pb-0"
        >
          {SECTIONS.map((item) => {
            const isActive = section === item.id
            return (
              <button
                key={item.id}
                type="button"
                onClick={() => setSection(item.id)}
                aria-current={isActive ? "page" : undefined}
                className={cn(
                  "flex shrink-0 items-center gap-2.5 rounded-md border px-2.5 py-2 text-left text-[13px] transition-colors",
                  isActive
                    ? "border-line bg-selected text-ink"
                    : "border-transparent text-ink-2 hover:bg-cream/60 hover:text-ink"
                )}
              >
                <item.icon
                  className={cn("h-4 w-4 shrink-0", isActive ? "text-gold-deep" : "text-muted")}
                  strokeWidth={1.6}
                />
                <span className="truncate">{item.label}</span>
              </button>
            )
          })}
        </nav>

        <div className="min-w-0">
          {section === "profile" && (
            <section className="rounded-lg border border-line bg-cream">
              <div className="border-b border-line px-5 py-3">
                <h2 className="text-lg text-ink">Profile</h2>
                <p className="mt-0.5 text-[12px] text-muted">
                  The email on the account and the name shown on your receipts.
                </p>
              </div>
              <div className="p-5">
                <form
                  onSubmit={profileForm.handleSubmit(onProfileSubmit)}
                  className="max-w-md space-y-5"
                >
                  <div>
                    <div className="flex items-center justify-between gap-3">
                      <label htmlFor="profile-email" className="text-[13px] font-medium text-ink-2">
                        Email
                      </label>
                      {user.email_verified ? (
                        <Badge variant="success">Verified</Badge>
                      ) : (
                        <Badge variant="warning">Unverified</Badge>
                      )}
                    </div>
                    <Input
                      id="profile-email"
                      value={user.email}
                      disabled
                      className="mt-1.5"
                    />
                    {!user.email_verified && (
                      <button
                        type="button"
                        onClick={onResendVerification}
                        disabled={resendLoading}
                        className="mt-2 text-[12px] text-gold-deep u-link disabled:opacity-50"
                      >
                        {resendLoading ? "Sending the email" : "Resend the verification email"}
                      </button>
                    )}
                  </div>

                  <div>
                    <label
                      htmlFor="profile-display-name"
                      className="text-[13px] font-medium text-ink-2"
                    >
                      Display name
                    </label>
                    <Input
                      id="profile-display-name"
                      {...profileForm.register("display_name")}
                      placeholder="Your name"
                      autoComplete="name"
                      className="mt-1.5"
                      error={profileForm.formState.errors.display_name?.message}
                    />
                  </div>

                  <div className="flex flex-wrap items-center gap-3 border-t border-line pt-5">
                    <Button
                      type="submit"
                      loading={profileLoading}
                      disabled={!profileForm.formState.isDirty}
                    >
                      Save profile
                    </Button>
                    <p className="text-[12px] text-muted">
                      {profileForm.formState.isDirty ? "Unsaved changes" : "Saved"}
                    </p>
                  </div>
                </form>
              </div>
            </section>
          )}

          {section === "security" && (
            <section className="rounded-lg border border-line bg-cream">
              <div className="border-b border-line px-5 py-3">
                <h2 className="text-lg text-ink">Security</h2>
                <p className="mt-0.5 text-[12px] text-muted">
                  Change the password for this account.
                </p>
              </div>
              <div className="p-5">
                <form
                  onSubmit={passwordForm.handleSubmit(onPasswordSubmit)}
                  className="max-w-md space-y-5"
                >
                  <div>
                    <label
                      htmlFor="current-password"
                      className="text-[13px] font-medium text-ink-2"
                    >
                      Current password
                    </label>
                    <Input
                      id="current-password"
                      type="password"
                      autoComplete="current-password"
                      {...passwordForm.register("current_password")}
                      placeholder="Enter your current password"
                      className="mt-1.5"
                      error={passwordForm.formState.errors.current_password?.message}
                    />
                  </div>

                  <div className="border-t border-line pt-5">
                    <label htmlFor="new-password" className="text-[13px] font-medium text-ink-2">
                      New password
                    </label>
                    <Input
                      id="new-password"
                      type="password"
                      autoComplete="new-password"
                      {...passwordForm.register("new_password")}
                      placeholder="At least 8 characters"
                      className="mt-1.5"
                      error={passwordForm.formState.errors.new_password?.message}
                    />
                  </div>

                  <div>
                    <label
                      htmlFor="confirm-password"
                      className="text-[13px] font-medium text-ink-2"
                    >
                      Repeat new password
                    </label>
                    <Input
                      id="confirm-password"
                      type="password"
                      autoComplete="new-password"
                      {...passwordForm.register("confirm_password")}
                      placeholder="Repeat the new password"
                      className="mt-1.5"
                      error={passwordForm.formState.errors.confirm_password?.message}
                    />
                  </div>

                  <div className="flex flex-wrap items-center gap-3 border-t border-line pt-5">
                    <Button type="submit" loading={passwordLoading}>
                      Change password
                    </Button>
                    <p className="text-[12px] text-muted">
                      You will be signed out after the change.
                    </p>
                  </div>
                </form>
              </div>
            </section>
          )}

          {section === "preferences" && (
            <section className="rounded-lg border border-line bg-cream">
              <div className="border-b border-line px-5 py-3">
                <h2 className="text-lg text-ink">Preferences</h2>
                <p className="mt-0.5 text-[12px] text-muted">
                  How bill amounts are displayed across the app.
                </p>
              </div>
              <div className="p-5">
                <p className="text-[13px] font-medium text-ink-2">Theme</p>
                <div className="mt-2.5">
                  <ThemePicker />
                </div>
                <p className="mt-2 max-w-md text-[12px] leading-relaxed text-muted">
                  System follows the setting on this device. The choice is saved in this browser.
                </p>

                <div className="mt-6 border-t border-line pt-5">
                  <p className="text-[13px] font-medium text-ink-2">Display currency</p>
                </div>
                <div
                  role="group"
                  aria-label="Display currency"
                  className="mt-2.5 overflow-hidden rounded-lg border border-line"
                >
                  {CURRENCY_CODES.map((code) => {
                    const currency = CURRENCIES[code]
                    const isSelected = activeCurrency === code
                    return (
                      <button
                        key={code}
                        type="button"
                        aria-pressed={isSelected}
                        onClick={() => setPreferredCurrency(code)}
                        className={cn(
                          "flex w-full items-center gap-3 border-b border-line px-4 py-2.5 text-left transition-colors last:border-b-0",
                          isSelected ? "bg-paper-2" : "hover:bg-paper-2/60"
                        )}
                      >
                        <span className="num w-9 shrink-0 text-[13px] text-ink-2">
                          {currency.symbol}
                        </span>
                        <span className="flex-1 text-[13px] text-ink">{currency.name}</span>
                        <span className="num text-[11px] text-muted">{code}</span>
                        {isSelected && (
                          <Check className="h-4 w-4 shrink-0 text-gold-deep" strokeWidth={1.8} />
                        )}
                      </button>
                    )
                  })}
                </div>
                <p className="mt-3 max-w-md text-[12px] leading-relaxed text-muted">
                  Bill totals stay in the bill's local currency on the server. The chosen
                  currency only changes the figure you see, converted at the live rate. Saved
                  in this browser.
                </p>
              </div>
            </section>
          )}

          {section === "danger" && (
            <section className="rounded-lg border border-line bg-cream">
              <div className="flex items-start gap-2.5 border-b border-line px-5 py-3">
                <TriangleAlert className="mt-0.5 h-4 w-4 shrink-0 text-rust" strokeWidth={1.6} />
                <div>
                  <h2 className="text-lg text-ink">Danger zone</h2>
                  <p className="mt-0.5 text-[12px] text-muted">Actions that cannot be undone.</p>
                </div>
              </div>
              <div className="p-5">
                <p className="text-[14px] text-ink">Delete this account</p>
                <p className="mt-1 max-w-md text-[13px] leading-relaxed text-ink-2">
                  This permanently deletes your account and all associated data. You will be signed
                  out immediately.
                </p>

                {deleteConfirmOpen ? (
                  <form
                    onSubmit={deleteForm.handleSubmit(onDeleteSubmit)}
                    className="mt-5 max-w-sm space-y-4 border-t border-line pt-5"
                  >
                    <div>
                      <label
                        htmlFor="delete-password"
                        className="text-[13px] font-medium text-ink-2"
                      >
                        Confirm your password
                      </label>
                      <Input
                        id="delete-password"
                        type="password"
                        autoComplete="current-password"
                        {...deleteForm.register("password")}
                        placeholder="Enter your password"
                        className="mt-1.5"
                        error={deleteForm.formState.errors.password?.message}
                      />
                    </div>
                    <div className="flex flex-wrap items-center gap-2">
                      <Button type="submit" variant="danger" loading={deleteLoading}>
                        Delete account
                      </Button>
                      <Button
                        type="button"
                        variant="ghost"
                        onClick={() => {
                          setDeleteConfirmOpen(false)
                          deleteForm.reset()
                        }}
                      >
                        Cancel
                      </Button>
                    </div>
                  </form>
                ) : (
                  <Button
                    type="button"
                    variant="secondary"
                    className="mt-4 border-line-strong text-rust hover:bg-paper-2"
                    onClick={() => setDeleteConfirmOpen(true)}
                  >
                    Delete account
                  </Button>
                )}
              </div>
            </section>
          )}
        </div>
      </div>
    </div>
  )
}
