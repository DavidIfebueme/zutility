"use client"

import * as React from "react"
import Link from "next/link"
import { usePathname, useRouter } from "next/navigation"
import {
  ArrowLeftRight,
  CreditCard,
  History,
  Home,
  LogOut,
  Menu,
  Settings,
  Store,
  X,
} from "lucide-react"
import { cn } from "@/lib/utils"
import { useAuthStore } from "@/store/auth"
import { useOrderStore } from "@/store/order"
import { NotificationDropdown } from "@/components/ui/notification-dropdown"
import { Button } from "@/components/ui/button"
import { RateTicker } from "@/components/ui/rate-ticker"
import { WordmarkLink } from "@/components/ui/wordmark"
import { ThemeToggle } from "@/components/theme-provider"

const NAV_ITEMS = [
  { href: "/dashboard", label: "Overview", icon: Home },
  { href: "/pay", label: "Pay a bill", icon: CreditCard, activeOrderDot: true },
  { href: "/history", label: "History", icon: History },
  { href: "/settings", label: "Settings", icon: Settings },
]

const SOON_ITEMS = [
  { href: "/otc", label: "OTC off-ramp", icon: ArrowLeftRight },
  { href: "/p2p", label: "P2P market", icon: Store },
]

function NavLink({
  href,
  label,
  icon: Icon,
  soon,
  dot,
  onNavigate,
}: {
  href: string
  label: string
  icon: React.ComponentType<{ className?: string; strokeWidth?: number }>
  soon?: boolean
  dot?: boolean
  onNavigate?: () => void
}) {
  const pathname = usePathname()
  const isActive = pathname === href || pathname.startsWith(`${href}/`)

  return (
    <Link
      href={href}
      onClick={onNavigate}
      className={cn(
        "group/nav flex items-center gap-2.5 rounded-md px-2.5 py-2 text-[13px] transition-colors",
        isActive
          ? "bg-selected text-ink shadow-[inset_0_0_0_1px_var(--color-line)]"
          : "text-ink-2 hover:bg-cream/60 hover:text-ink"
      )}
    >
      <Icon
        className={cn(
          "h-4 w-4 shrink-0 transition-transform duration-200 group-hover/nav:translate-x-0.5",
          isActive ? "text-gold-deep" : "text-muted"
        )}
        strokeWidth={1.7}
      />
      <span className="flex-1 truncate">{label}</span>
      {soon && <span className="text-[10px] uppercase tracking-wider text-muted">Soon</span>}
      {dot && <span className="h-1.5 w-1.5 rounded-full bg-gold" aria-label="Open order" />}
    </Link>
  )
}

function SidebarBody({ onNavigate }: { onNavigate?: () => void }) {
  const { user, logout } = useAuthStore()
  const { activeOrder } = useOrderStore()

  return (
    <>
      <div className="flex h-16 items-center border-b border-line px-4">
        <WordmarkLink />
      </div>

      <div className="border-b border-line px-4 py-3">
        <RateTicker />
      </div>

      <nav className="flex-1 space-y-1 overflow-y-auto px-2.5 py-4" aria-label="Account">
        {NAV_ITEMS.map((item) => (
          <NavLink
            key={item.href}
            {...item}
            dot={Boolean(item.activeOrderDot && activeOrder)}
            onNavigate={onNavigate}
          />
        ))}

        <p className="label-xs px-2.5 pb-1 pt-5 text-muted">Coming later</p>
        {SOON_ITEMS.map((item) => (
          <NavLink key={item.href} {...item} soon onNavigate={onNavigate} />
        ))}
      </nav>

      <div className="border-t border-line px-4 py-3">
        <div className="flex items-center gap-2">
          <div className="min-w-0 flex-1">
            <p className="truncate text-[13px] font-medium text-ink">
              {user?.display_name || "Account"}
            </p>
            <p className="truncate text-[12px] text-muted">{user?.email}</p>
          </div>
          <button
            onClick={logout}
            className="flex h-8 w-8 items-center justify-center rounded-md text-muted transition-colors hover:bg-cream hover:text-rust"
            aria-label="Sign out"
          >
            <LogOut className="h-4 w-4" strokeWidth={1.7} />
          </button>
        </div>
      </div>
    </>
  )
}

export default function AppLayout({ children }: { children: React.ReactNode }) {
  const router = useRouter()
  const { isAuthenticated, hasHydrated } = useAuthStore()
  const [menuOpen, setMenuOpen] = React.useState(false)

  React.useEffect(() => {
    if (hasHydrated && !isAuthenticated) {
      router.push("/login")
    }
  }, [hasHydrated, isAuthenticated, router])

  if (!hasHydrated) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-paper">
        <div className="h-6 w-6 animate-spin rounded-full border-2 border-line-strong border-t-ink" />
      </div>
    )
  }

  if (!isAuthenticated) {
    return (
      <div className="flex min-h-screen flex-col items-center justify-center gap-4 bg-paper px-6 text-center">
        <p className="label-xs text-muted">Session</p>
        <p className="max-w-xs text-lg text-ink">Sign in to see your orders.</p>
        <Button asChild>
          <Link href="/login">Sign in</Link>
        </Button>
      </div>
    )
  }

  return (
    <div className="min-h-screen bg-paper">
      <aside className="fixed inset-y-0 left-0 z-30 hidden w-60 flex-col border-r border-line bg-paper-2 lg:flex">
        <SidebarBody />
      </aside>

      <div className="lg:pl-60">
        <header className="sticky top-0 z-20 hidden h-14 items-center justify-end gap-1 border-b border-line bg-paper/90 px-6 backdrop-blur-sm lg:flex">
          <ThemeToggle />
          <NotificationDropdown />
        </header>

        <header className="sticky top-0 z-20 flex h-16 items-center justify-between gap-3 border-b border-line bg-paper/90 px-4 backdrop-blur-sm lg:hidden">
          <div className="flex items-center gap-2">
            <button
              onClick={() => setMenuOpen(true)}
              className="flex h-9 w-9 items-center justify-center rounded-md text-ink"
              aria-label="Open menu"
            >
              <Menu className="h-5 w-5" strokeWidth={1.7} />
            </button>
            <WordmarkLink />
          </div>
          <div className="flex items-center gap-1">
            <RateTicker className="hidden sm:flex" />
            <ThemeToggle />
            <NotificationDropdown />
          </div>
        </header>

        {menuOpen && (
          <div className="fixed inset-0 z-50 lg:hidden">
            <button
              className="absolute inset-0 bg-vault/40"
              onClick={() => setMenuOpen(false)}
              aria-label="Close menu"
            />
            <div className="relative flex h-full w-64 flex-col border-r border-line bg-paper-2">
              <button
                onClick={() => setMenuOpen(false)}
                className="absolute right-3 top-4 z-10 flex h-8 w-8 items-center justify-center rounded-md text-muted"
                aria-label="Close menu"
              >
                <X className="h-4 w-4" strokeWidth={1.7} />
              </button>
              <SidebarBody onNavigate={() => setMenuOpen(false)} />
            </div>
          </div>
        )}

        <main className="mx-auto w-full max-w-4xl px-5 py-8 sm:px-8 sm:py-10">{children}</main>
      </div>
    </div>
  )
}
