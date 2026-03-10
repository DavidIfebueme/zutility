"use client"

import { useState, useEffect, useRef } from 'react'
import { OrderStatus, OrderStreamEvent } from '../types'
import { toast } from 'sonner'

interface UseOrderStreamReturn {
  status: OrderStatus
  confirmations: number
  latestEvent: OrderStreamEvent | null
  isConnected: boolean
}

export function useOrderStream(orderId: string | null, accessToken: string | null): UseOrderStreamReturn {
  const [status, setStatus] = useState<OrderStatus>('awaiting_payment')
  const [confirmations, setConfirmations] = useState(0)
  const [latestEvent, setLatestEvent] = useState<OrderStreamEvent | null>(null)
  const [isConnected, setIsConnected] = useState(false)
  
  const wsRef = useRef<WebSocket | null>(null)
  const reconnectTimeoutRef = useRef<NodeJS.Timeout | null>(null)
  const backoffRef = useRef(1000)

  useEffect(() => {
    if (!orderId || !accessToken) return

    let isMounted = true

    const connect = () => {
      // In a real app, use NEXT_PUBLIC_WS_URL
      // const wsUrl = `${process.env.NEXT_PUBLIC_WS_URL}/api/v1/orders/${orderId}/stream?token=${accessToken}`
      
      // Mocking WebSocket behavior for now
      console.log(`[WS] Connecting to stream for order ${orderId}`)
      
      // Simulate connection
      setTimeout(() => {
        if (!isMounted) return
        setIsConnected(true)
        backoffRef.current = 1000
        
        // Simulate events
        // 1. Payment detected after 5s
        setTimeout(() => {
          if (!isMounted) return
          setStatus('payment_detected')
          setConfirmations(0)
          setLatestEvent({ event: 'payment_detected', confirmations: 0, required: 10 })
        }, 5000)

        // 2. Confirmations every 2s
        let confs = 0
        const confInterval = setInterval(() => {
          if (!isMounted || confs >= 10) {
            clearInterval(confInterval)
            return
          }
          confs++
          setConfirmations(confs)
          setLatestEvent({ event: 'confirmation', confirmations: confs, required: 10 })
          
          if (confs === 10) {
            setStatus('payment_confirmed')
            setLatestEvent({ event: 'payment_confirmed', confirmations: 10 })
            
            // 3. Dispatching
            setTimeout(() => {
              if (!isMounted) return
              setStatus('utility_dispatching')
              setLatestEvent({ event: 'dispatching' })
              
              // 4. Completed
              setTimeout(() => {
                if (!isMounted) return
                setStatus('completed')
                setLatestEvent({ event: 'completed', delivery_token: '1234-5678-9012-3456', reference: 'REF-123' })
                setIsConnected(false) // Terminal state
              }, 2000)
            }, 2000)
          }
        }, 2000)
      }, 500)
    }

    connect()

    const ws = wsRef.current
    const timeout = reconnectTimeoutRef.current

    return () => {
      isMounted = false
      if (ws) {
        ws.close()
      }
      if (timeout) {
        clearTimeout(timeout)
      }
    }
  }, [orderId, accessToken])

  return { status, confirmations, latestEvent, isConnected }
}
