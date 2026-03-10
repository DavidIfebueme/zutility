"use client"

import * as React from "react"
import Link from "next/link"
import { usePathname } from "next/navigation"
import { motion, AnimatePresence } from "motion/react"
import { 
  Home, 
  CreditCard, 
  ArrowRightLeft, 
  Store, 
  History, 
  Settings, 
  LogOut,
  Menu,
  Bell
} from "lucide-react"
import { cn } from "@/lib/utils"
import { useAuthStore } from "@/store/auth"
import { useOrderStore } from "@/store/order"
import { RateTicker } from "@/components/ui/rate-ticker"

const NAV_ITEMS = [
  { href: "/dashboard", label: "Home", icon: Home },
  { href: "/pay", label: "Pay Utilities", icon: CreditCard, badge: true },
  { href: "/otc", label: "OTC Off-ramp", icon: ArrowRightLeft, soon: true },
  { href: "/p2p", label: "P2P Market", icon: Store, soon: true },
  { href: "/history", label: "History", icon: History },
  { href: "/settings", label: "Settings", icon: Settings },
]

export default function AppLayout({ children }: { children: React.ReactNode }) {
  const pathname = usePathname()
  const { user, logout } = useAuthStore()
  const { activeOrder } = useOrderStore()
  const [isMobileMenuOpen, setIsMobileMenuOpen] = React.useState(false)

  // Close mobile menu on route change
  React.useEffect(() => {
    setIsMobileMenuOpen(false)
  }, [pathname])

  return (
    <div className="flex min-h-screen bg-bg-void text-text-primary">
      {/* Desktop Sidebar */}
      <aside className="hidden w-64 flex-col border-r border-border-subtle bg-bg-surface xl:flex">
        <div className="flex h-16 items-center px-6">
          <Link href="/dashboard" className="font-dela text-xl tracking-tight">
            <span className="text-accent-zec">z</span>utility
          </Link>
        </div>
        
        <div className="px-6 py-4 border-b border-border-subtle">
          <RateTicker />
        </div>

        <nav className="flex-1 space-y-1 px-3 py-4">
          {NAV_ITEMS.map((item) => {
            const isActive = pathname.startsWith(item.href)
            return (
              <Link
                key={item.href}
                href={item.href}
                className={cn(
                  "group flex items-center rounded-md px-3 py-2.5 text-sm font-medium transition-colors",
                  isActive
                    ? "bg-bg-elevated text-text-primary border-l-2 border-accent-zec"
                    : "text-text-secondary hover:bg-bg-elevated/50 hover:text-text-primary"
                )}
              >
                <item.icon
                  className={cn(
                    "mr-3 h-5 w-5 flex-shrink-0 transition-colors",
                    isActive ? "text-accent-zec" : "text-text-muted group-hover:text-text-secondary"
                  )}
                />
                <span className="flex-1">{item.label}</span>
                {item.badge && activeOrder && (
                  <span className="ml-auto inline-block h-2 w-2 rounded-full bg-accent-zec" />
                )}
                {item.soon && (
                  <span className="ml-auto rounded bg-accent-zec/10 px-1.5 py-0.5 text-[10px] font-semibold text-accent-zec uppercase tracking-wider">
                    Soon
                  </span>
                )}
              </Link>
            )
          })}
        </nav>

        <div className="border-t border-border-subtle p-4">
          <div className="flex items-center justify-between px-3 py-2">
            <div className="flex flex-col truncate">
              <span className="text-sm font-medium text-text-primary truncate">
                {user?.displayName || 'User'}
              </span>
              <span className="text-xs text-text-muted truncate">
                {user?.email}
              </span>
            </div>
            <button
              onClick={logout}
              className="text-text-muted hover:text-accent-red transition-colors"
              aria-label="Logout"
            >
              <LogOut className="h-5 w-5" />
            </button>
          </div>
        </div>
      </aside>

      {/* Mobile Header */}
      <div className="flex flex-1 flex-col xl:hidden">
        <header className="sticky top-0 z-40 flex h-16 items-center justify-between border-b border-border-subtle bg-bg-surface/80 px-4 backdrop-blur-md">
          <div className="flex items-center gap-3">
            <button
              onClick={() => setIsMobileMenuOpen(true)}
              className="text-text-secondary hover:text-text-primary"
            >
              <Menu className="h-6 w-6" />
            </button>
            <Link href="/dashboard" className="font-dela text-lg tracking-tight">
              <span className="text-accent-zec">z</span>utility
            </Link>
          </div>
          <div className="flex items-center gap-4">
            <RateTicker className="hidden sm:flex" />
            <button className="text-text-secondary hover:text-text-primary relative">
              <Bell className="h-5 w-5" />
              {activeOrder && (
                <span className="absolute -top-1 -right-1 h-2 w-2 rounded-full bg-accent-zec" />
              )}
            </button>
          </div>
        </header>

        {/* Mobile Menu Overlay */}
        <AnimatePresence>
          {isMobileMenuOpen && (
            <>
              <motion.div
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                exit={{ opacity: 0 }}
                className="fixed inset-0 z-40 bg-bg-void/80 backdrop-blur-sm"
                onClick={() => setIsMobileMenuOpen(false)}
              />
              <motion.div
                initial={{ x: "-100%" }}
                animate={{ x: 0 }}
                exit={{ x: "-100%" }}
                transition={{ type: "spring", bounce: 0, duration: 0.3 }}
                className="fixed inset-y-0 left-0 z-50 w-64 bg-bg-surface border-r border-border-subtle flex flex-col"
              >
                <div className="flex h-16 items-center px-6 border-b border-border-subtle">
                  <span className="font-dela text-xl tracking-tight">
                    <span className="text-accent-zec">z</span>utility
                  </span>
                </div>
                
                <div className="px-6 py-4 border-b border-border-subtle sm:hidden">
                  <RateTicker />
                </div>

                <nav className="flex-1 space-y-1 px-3 py-4 overflow-y-auto">
                  {NAV_ITEMS.map((item) => {
                    const isActive = pathname.startsWith(item.href)
                    return (
                      <Link
                        key={item.href}
                        href={item.href}
                        className={cn(
                          "group flex items-center rounded-md px-3 py-3 text-sm font-medium transition-colors",
                          isActive
                            ? "bg-bg-elevated text-text-primary border-l-2 border-accent-zec"
                            : "text-text-secondary hover:bg-bg-elevated/50 hover:text-text-primary"
                        )}
                      >
                        <item.icon
                          className={cn(
                            "mr-3 h-5 w-5 flex-shrink-0",
                            isActive ? "text-accent-zec" : "text-text-muted"
                          )}
                        />
                        <span className="flex-1">{item.label}</span>
                        {item.soon && (
                          <span className="ml-auto rounded bg-accent-zec/10 px-1.5 py-0.5 text-[10px] font-semibold text-accent-zec uppercase tracking-wider">
                            Soon
                          </span>
                        )}
                      </Link>
                    )
                  })}
                </nav>

                <div className="border-t border-border-subtle p-4">
                  <div className="flex items-center justify-between px-3 py-2">
                    <div className="flex flex-col truncate">
                      <span className="text-sm font-medium text-text-primary truncate">
                        {user?.displayName || 'User'}
                      </span>
                      <span className="text-xs text-text-muted truncate">
                        {user?.email}
                      </span>
                    </div>
                    <button
                      onClick={logout}
                      className="text-text-muted hover:text-accent-red transition-colors"
                    >
                      <LogOut className="h-5 w-5" />
                    </button>
                  </div>
                </div>
              </motion.div>
            </>
          )}
        </AnimatePresence>

        {/* Main Content */}
        <main className="flex-1 overflow-y-auto">
          <div className="mx-auto max-w-5xl p-4 sm:p-6 lg:p-8">
            <AnimatePresence mode="wait">
              <motion.div
                key={pathname}
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: -20 }}
                transition={{ duration: 0.2 }}
              >
                {children}
              </motion.div>
            </AnimatePresence>
          </div>
        </main>
      </div>

      {/* Desktop Main Content */}
      <main className="hidden xl:flex flex-1 flex-col overflow-y-auto">
        <div className="mx-auto w-full max-w-5xl p-8">
          <AnimatePresence mode="wait">
            <motion.div
              key={pathname}
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -20 }}
              transition={{ duration: 0.2 }}
            >
              {children}
            </motion.div>
          </AnimatePresence>
        </div>
      </main>
    </div>
  )
}
