import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import { apiPost } from '@/lib/api'
import { detectCurrency, type CurrencyCode } from '@/lib/currency'

export interface AuthUser {
  id: string
  email: string
  display_name: string | null
  email_verified: boolean
}

interface AuthState {
  user: AuthUser | null
  isAuthenticated: boolean
  preferredCurrency: CurrencyCode | null
  hasHydrated: boolean
  setUser: (user: AuthUser) => void
  logout: () => void
  setPreferredCurrency: (currency: CurrencyCode) => void
  setHasHydrated: (v: boolean) => void
}

function clearAuthCookies() {
  const domain = window.location.hostname.replace(/^www\./, '')
  const paths = ['/', '/api', '/api/v1/auth/refresh']
  const cookies = ['csrf_token', 'access_token', 'refresh_token']
  for (const name of cookies) {
    for (const path of paths) {
      document.cookie = `${name}=; expires=Thu, 01 Jan 1970 00:00:00 UTC; path=${path}; domain=.${domain};`
      document.cookie = `${name}=; expires=Thu, 01 Jan 1970 00:00:00 UTC; path=${path};`
    }
  }
}

export const useAuthStore = create<AuthState>()(
  persist(
    (set, get) => ({
      user: null,
      isAuthenticated: false,
      preferredCurrency: null,
      hasHydrated: false,
      setUser: (user: AuthUser) => set({
        user,
        isAuthenticated: true,
        preferredCurrency: get().preferredCurrency || detectCurrency(),
      }),
      logout: () => {
        apiPost('/api/v1/auth/logout', {}).catch(() => {})
        clearAuthCookies()
        set({ user: null, isAuthenticated: false, preferredCurrency: null })
      },
      setPreferredCurrency: (currency: CurrencyCode) => set({ preferredCurrency: currency }),
      setHasHydrated: (v: boolean) => set({ hasHydrated: v }),
    }),
    {
      name: 'zutility-auth',
      partialize: (state) => ({
        user: state.user,
        isAuthenticated: state.isAuthenticated,
        preferredCurrency: state.preferredCurrency,
      }),
      onRehydrateStorage: () => (state) => {
        state?.setHasHydrated(true)
      },
    }
  )
)
