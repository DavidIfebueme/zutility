import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import { apiPost, clearSessionFreshness } from '@/lib/api'
import { useOrderStore } from '@/store/order'
import type { CurrencyCode } from '@/lib/currency'
import { isCurrencyCode } from '@/lib/currency'
import { toast } from 'sonner'

export interface AuthUser {
  id: string
  email: string
  display_name: string | null
  email_verified: boolean
  preferred_currency?: CurrencyCode
}

interface AuthState {
  user: AuthUser | null
  isAuthenticated: boolean
  preferredCurrency: CurrencyCode | null
  hasHydrated: boolean
  setUser: (user: AuthUser) => void
  logout: () => Promise<boolean>
  setHasHydrated: (v: boolean) => void
}

const PERSIST_VERSION = 2

type PersistedAuth = {
  user: AuthUser | null
  isAuthenticated: boolean
}

function clearAuthCookies() {
  const domain = window.location.hostname.replace(/^www\./, '')
  const paths = ['/', '/api', '/api/v1/auth']
  const cookies = ['csrf_token', 'access_token', 'refresh_token', 'session']
  for (const name of cookies) {
    for (const path of paths) {
      document.cookie = `${name}=; expires=Thu, 01 Jan 1970 00:00:00 UTC; path=${path}; domain=.${domain};`
      document.cookie = `${name}=; expires=Thu, 01 Jan 1970 00:00:00 UTC; path=${path};`
    }
  }
}

function preferredFromUser(user: AuthUser): CurrencyCode | null {
  const value = user.preferred_currency
  return value && isCurrencyCode(value) ? value : null
}

export const useAuthStore = create<AuthState>()(
  persist(
    (set) => ({
      user: null,
      isAuthenticated: false,
      preferredCurrency: null,
      hasHydrated: false,
      setUser: (user: AuthUser) => set({
        user,
        isAuthenticated: true,
        preferredCurrency: preferredFromUser(user),
      }),
      logout: async () => {
        try {
          await apiPost('/api/v1/auth/logout', {})
        } catch {
          toast.error('Could not sign out. Check your connection and try again.')
          return false
        }
        useOrderStore.getState().clearActiveOrder()
        clearSessionFreshness()
        clearAuthCookies()
        set({ user: null, isAuthenticated: false, preferredCurrency: null })
        return true
      },
      setHasHydrated: (v: boolean) => set({ hasHydrated: v }),
    }),
    {
      name: 'zutility-auth',
      version: PERSIST_VERSION,
      partialize: (state) => ({
        user: state.user,
        isAuthenticated: state.isAuthenticated,
      }),
      migrate: (persisted) => {
        const state = (persisted ?? {}) as Record<string, unknown>
        delete state.preferredCurrency
        return {
          user: (state.user as AuthUser | null) ?? null,
          isAuthenticated: state.isAuthenticated === true,
        } satisfies PersistedAuth
      },
      onRehydrateStorage: () => (state) => {
        state?.setHasHydrated(true)
      },
    }
  )
)
