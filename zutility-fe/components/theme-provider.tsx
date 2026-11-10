"use client"

import * as React from "react"
import { Monitor, Moon, Sun } from "lucide-react"
import { Toaster } from "sonner"
import { cn } from "@/lib/utils"

export type Theme = "light" | "dark" | "system"
type ResolvedTheme = "light" | "dark"

const STORAGE_KEY = "zutility-theme"
const THEME_EVENT = "zutility-theme-change"
const DARK_QUERY = "(prefers-color-scheme: dark)"

interface ThemeContextValue {
  theme: Theme
  resolvedTheme: ResolvedTheme
  setTheme: (theme: Theme) => void
}

const ThemeContext = React.createContext<ThemeContextValue | null>(null)

function subscribeTheme(callback: () => void) {
  window.addEventListener(THEME_EVENT, callback)
  window.addEventListener("storage", callback)
  return () => {
    window.removeEventListener(THEME_EVENT, callback)
    window.removeEventListener("storage", callback)
  }
}

function getThemeSnapshot(): Theme {
  const stored = window.localStorage.getItem(STORAGE_KEY)
  return stored === "light" || stored === "dark" || stored === "system" ? stored : "system"
}

function subscribeSystem(callback: () => void) {
  const media = window.matchMedia(DARK_QUERY)
  media.addEventListener("change", callback)
  return () => media.removeEventListener("change", callback)
}

function getSystemSnapshot(): boolean {
  return window.matchMedia(DARK_QUERY).matches
}

export function ThemeProvider({ children }: { children: React.ReactNode }) {
  const theme = React.useSyncExternalStore(subscribeTheme, getThemeSnapshot, () => "system" as Theme)
  const systemDark = React.useSyncExternalStore(subscribeSystem, getSystemSnapshot, () => false)

  const resolvedTheme: ResolvedTheme =
    theme === "system" ? (systemDark ? "dark" : "light") : theme

  React.useEffect(() => {
    const root = document.documentElement
    root.classList.toggle("dark", resolvedTheme === "dark")
    root.style.colorScheme = resolvedTheme
  }, [resolvedTheme])

  const setTheme = React.useCallback((next: Theme) => {
    window.localStorage.setItem(STORAGE_KEY, next)
    window.dispatchEvent(new Event(THEME_EVENT))
  }, [])

  const value = React.useMemo(
    () => ({ theme, resolvedTheme, setTheme }),
    [theme, resolvedTheme, setTheme]
  )

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>
}

export function useTheme(): ThemeContextValue {
  const context = React.useContext(ThemeContext)
  if (!context) {
    throw new Error("useTheme must be used inside ThemeProvider")
  }
  return context
}

const NEXT_THEME: Record<Theme, Theme> = {
  system: "light",
  light: "dark",
  dark: "system",
}

export function ThemeToggle({ className }: { className?: string }) {
  const { theme, resolvedTheme, setTheme } = useTheme()
  const Icon = theme === "system" ? Monitor : resolvedTheme === "dark" ? Moon : Sun

  return (
    <button
      type="button"
      onClick={() => setTheme(NEXT_THEME[theme])}
      className={cn(
        "flex h-8 w-8 items-center justify-center rounded-md text-muted transition-colors duration-150 hover:bg-paper-3/60 hover:text-ink active:scale-95",
        className
      )}
      aria-label={`Theme: ${theme}. Switch to ${NEXT_THEME[theme]}.`}
      title={`Theme: ${theme}`}
    >
      <Icon className="h-4 w-4" strokeWidth={1.7} />
    </button>
  )
}

export function ThemePicker() {
  const { theme, setTheme } = useTheme()
  const options: { value: Theme; label: string; icon: typeof Sun }[] = [
    { value: "light", label: "Light", icon: Sun },
    { value: "dark", label: "Dark", icon: Moon },
    { value: "system", label: "System", icon: Monitor },
  ]

  return (
    <div
      role="radiogroup"
      aria-label="Theme"
      className="inline-flex rounded-lg border border-line bg-cream p-0.5"
    >
      {options.map((option) => {
        const isActive = theme === option.value
        return (
          <button
            key={option.value}
            type="button"
            role="radio"
            aria-checked={isActive}
            onClick={() => setTheme(option.value)}
            className={cn(
              "flex items-center gap-1.5 rounded-md px-2.5 py-1.5 text-[12px] transition-colors duration-150",
              isActive ? "bg-paper-3 text-ink" : "text-muted hover:text-ink"
            )}
          >
            <option.icon className="h-3.5 w-3.5" strokeWidth={1.7} />
            {option.label}
          </button>
        )
      })}
    </div>
  )
}

export function AppToaster() {
  const { resolvedTheme } = useTheme()

  return (
    <Toaster
      theme={resolvedTheme}
      position="bottom-right"
      toastOptions={{
        style: {
          background: "var(--cream)",
          border: "1px solid var(--line)",
          color: "var(--ink)",
          borderRadius: "8px",
        },
      }}
    />
  )
}
