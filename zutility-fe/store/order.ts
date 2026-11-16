import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import { CreateOrderResponse } from '@/lib/types'

interface OrderState {
  activeOrder: CreateOrderResponse | null
  activeOrderUserId: string | null
  setActiveOrder: (order: CreateOrderResponse, userId: string | null) => void
  clearActiveOrder: () => void
}

export const useOrderStore = create<OrderState>()(
  persist(
    (set) => ({
      activeOrder: null,
      activeOrderUserId: null,
      setActiveOrder: (order, userId) =>
        set({ activeOrder: order, activeOrderUserId: userId }),
      clearActiveOrder: () => set({ activeOrder: null, activeOrderUserId: null }),
    }),
    {
      name: 'zutility-order',
    }
  )
)
