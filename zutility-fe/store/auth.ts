import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import { apiPost } from '@/lib/api'

export interface AuthUser {
  id: string
  email: string
  display_name: string | null
  email_verified: boolean
}

interface AuthState {
  user: AuthUser | null
  isAuthenticated: boolean
  setUser: (user: AuthUser) => void
  logout: () => void
}

export const useAuthStore = create<AuthState>()(
  persist(
    (set) => ({
      user: null,
      isAuthenticated: false,
      setUser: (user: AuthUser) => set({ user, isAuthenticated: true }),
      logout: () => {
        apiPost('/api/v1/auth/logout', {}).catch(() => {})
        set({ user: null, isAuthenticated: false })
      },
    }),
    {
      name: 'zutility-auth',
      partialize: (state) => ({ user: state.user, isAuthenticated: state.isAuthenticated }),
    }
  )
)
