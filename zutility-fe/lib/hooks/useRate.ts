"use client"

import { useState, useEffect } from 'react'
import { RateResponse } from '../types'

// Mock rate for now
const MOCK_RATE: RateResponse = {
  zec_ngn: "150000.00",
  zec_usd: "100.00",
  updated_at: new Date().toISOString(),
  valid_until: new Date(Date.now() + 15 * 60000).toISOString()
}

export function useRate() {
  const [rate, setRate] = useState<RateResponse | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [isError, setIsError] = useState(false)
  const [lastUpdated, setLastUpdated] = useState<string | null>(null)

  useEffect(() => {
    let isMounted = true

    const fetchRate = async () => {
      try {
        // In a real app, fetch from NEXT_PUBLIC_API_URL
        // const res = await fetch(`${process.env.NEXT_PUBLIC_API_URL}/api/v1/rates/current`)
        // const data = await res.json()
        
        // Mock delay
        await new Promise(resolve => setTimeout(resolve, 500))
        
        if (isMounted) {
          setRate(MOCK_RATE)
          setLastUpdated(new Date().toISOString())
          setIsError(false)
        }
      } catch (err) {
        if (isMounted) setIsError(true)
      } finally {
        if (isMounted) setIsLoading(false)
      }
    }

    fetchRate()
    const interval = setInterval(fetchRate, 60000) // 60s

    return () => {
      isMounted = false
      clearInterval(interval)
    }
  }, [])

  return { rate, isLoading, isError, lastUpdated }
}
