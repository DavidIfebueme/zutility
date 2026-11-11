import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import { CreateOrderResponse, OrderStatus } from '@/lib/types'

interface OrderState {
  activeOrder: CreateOrderResponse | null
  activeOrderUserId: string | null
  status: OrderStatus | null
  setActiveOrder: (order: CreateOrderResponse, userId: string | null) => void
  clearActiveOrder: () => void
  setStatus: (status: OrderStatus) => void
}

export const useOrderStore = create<OrderState>()(
  persist(
    (set) => ({
      activeOrder: null,
      activeOrderUserId: null,
      status: null,
      setActiveOrder: (order, userId) =>
        set({ activeOrder: order, activeOrderUserId: userId, status: 'awaiting_payment' }),
      clearActiveOrder: () => set({ activeOrder: null, activeOrderUserId: null, status: null }),
      setStatus: (status) => set({ status }),
    }),
    {
      name: 'zutility-order',
    }
  )
)
