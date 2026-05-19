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
  setUser: (user: AuthUser) => void
  logout: () => void
  setPreferredCurrency: (currency: CurrencyCode) => void
}

export const useAuthStore = create<AuthState>()(
  persist(
    (set, get) => ({
      user: null,
      isAuthenticated: false,
      preferredCurrency: null,
      setUser: (user: AuthUser) => set({
        user,
        isAuthenticated: true,
        preferredCurrency: get().preferredCurrency || detectCurrency(),
      }),
      logout: () => {
        apiPost('/api/v1/auth/logout', {}).catch(() => {})
        set({ user: null, isAuthenticated: false })
      },
      setPreferredCurrency: (currency: CurrencyCode) => set({ preferredCurrency: currency }),
    }),
    {
      name: 'zutility-auth',
      partialize: (state) => ({
        user: state.user,
        isAuthenticated: state.isAuthenticated,
        preferredCurrency: state.preferredCurrency,
      }),
    }
  )
)
