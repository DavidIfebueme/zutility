"use client"

import { useState, useEffect } from 'react'
import { apiGet } from '../api'
import { RateResponse } from '../types'

const TTL_MS = 60_000

let cachedRate: RateResponse | null = null
let cachedAt = 0
let inFlight: Promise<RateResponse | null> | null = null

function fetchSharedRate(): Promise<RateResponse | null> {
  if (inFlight) return inFlight

  inFlight = apiGet<RateResponse>('/api/v1/rates/current')
    .then((data) => {
      cachedRate = data
      cachedAt = Date.now()
      return data
    })
    .catch(() => null)
    .finally(() => {
      inFlight = null
    })

  return inFlight
}

export function useRate() {
  const [rate, setRate] = useState<RateResponse | null>(cachedRate)
  const [isLoading, setIsLoading] = useState(cachedRate === null)
  const [isError, setIsError] = useState(false)
  const [lastUpdated, setLastUpdated] = useState<string | null>(cachedRate?.updated_at ?? null)

  useEffect(() => {
    let active = true

    const load = async () => {
      if (cachedRate && Date.now() - cachedAt < TTL_MS) {
        setRate(cachedRate)
        setLastUpdated(cachedRate.updated_at)
        setIsLoading(false)
        return
      }

      const data = await fetchSharedRate()
      if (!active) return

      if (data) {
        setRate(data)
        setLastUpdated(data.updated_at)
        setIsError(false)
      } else {
        setIsError(true)
      }
      setIsLoading(false)
    }

    void load()
    const interval = setInterval(() => void load(), TTL_MS)
    return () => {
      active = false
      clearInterval(interval)
    }
  }, [])

  return { rate, isLoading, isError, lastUpdated }
}
