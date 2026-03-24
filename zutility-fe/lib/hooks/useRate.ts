"use client"

import { useState, useEffect } from 'react'
import { apiGet } from '../api'
import { RateResponse } from '../types'

export function useRate() {
  const [rate, setRate] = useState<RateResponse | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [isError, setIsError] = useState(false)
  const [lastUpdated, setLastUpdated] = useState<string | null>(null)

  useEffect(() => {
    let isMounted = true

    const fetchRate = async () => {
      try {
        const data = await apiGet<RateResponse>('/api/v1/rates/current')
        
        if (isMounted) {
          setRate(data)
          setLastUpdated(data.updated_at)
          setIsError(false)
        }
      } catch {
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
