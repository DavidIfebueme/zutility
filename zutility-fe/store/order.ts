import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import { CreateOrderResponse, OrderStatus } from '@/lib/types'

interface OrderState {
  activeOrder: CreateOrderResponse | null
  status: OrderStatus | null
  setActiveOrder: (order: CreateOrderResponse) => void
  clearActiveOrder: () => void
  setStatus: (status: OrderStatus) => void
}

export const useOrderStore = create<OrderState>()(
  persist(
    (set) => ({
      activeOrder: null,
      status: null,
      setActiveOrder: (order) => set({ activeOrder: order, status: 'awaiting_payment' }),
      clearActiveOrder: () => set({ activeOrder: null, status: null }),
      setStatus: (status) => set({ status }),
    }),
    {
      name: 'zutility-order',
    }
  )
)
