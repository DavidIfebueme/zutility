"use client"

import { useState, useEffect, useRef } from 'react'
import { getWsBaseUrl } from '../api'
import { OrderStatus, OrderStreamEvent } from '../types'

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
    if (!orderId || !accessToken) {
      return
    }

    let isMounted = true
    let terminal = false

    const connect = () => {
      if (!isMounted || terminal) return

      const wsUrl = `${getWsBaseUrl()}/api/v1/orders/${orderId}/stream?token=${encodeURIComponent(accessToken)}`
      const socket = new WebSocket(wsUrl)
      wsRef.current = socket

      socket.onopen = () => {
        if (!isMounted) return
        setIsConnected(true)
        backoffRef.current = 1000
      }

      socket.onmessage = (message) => {
        if (!isMounted) return
        try {
          const event = JSON.parse(message.data) as OrderStreamEvent
          setLatestEvent(event)

          if (event.event === 'payment_detected') {
            setStatus('payment_detected')
            setConfirmations(event.confirmations)
          } else if (event.event === 'confirmation') {
            setStatus('payment_detected')
            setConfirmations(event.confirmations)
          } else if (event.event === 'payment_confirmed') {
            setStatus('payment_confirmed')
            setConfirmations(event.confirmations)
          } else if (event.event === 'dispatching') {
            setStatus('utility_dispatching')
          } else if (event.event === 'completed') {
            setStatus('completed')
            setIsConnected(false)
            terminal = true
          } else if (event.event === 'expired') {
            setStatus('expired')
            setIsConnected(false)
            terminal = true
          } else if (event.event === 'failed') {
            setStatus('failed')
            setIsConnected(false)
            terminal = true
          }
        } catch {
          setIsConnected(false)
        }
      }

      socket.onclose = () => {
        if (!isMounted) return
        setIsConnected(false)
        wsRef.current = null

        if (terminal) return

        const wait = backoffRef.current
        backoffRef.current = Math.min(wait * 2, 15000)
        reconnectTimeoutRef.current = setTimeout(connect, wait)
      }

      socket.onerror = () => {
        if (!isMounted) return
        setIsConnected(false)
      }
    }

    connect()

    return () => {
      isMounted = false
      terminal = true
      const ws = wsRef.current
      if (ws) {
        ws.close()
      }
      const timeout = reconnectTimeoutRef.current
      if (timeout) {
        clearTimeout(timeout)
      }
    }
  }, [orderId, accessToken])

  return { status, confirmations, latestEvent, isConnected }
}
