"use client"

import * as React from "react"

export function useShake(duration = 460) {
  const [shaking, setShaking] = React.useState(false)
  const timer = React.useRef<ReturnType<typeof setTimeout> | null>(null)

  const shake = React.useCallback(() => {
    if (timer.current) clearTimeout(timer.current)
    setShaking(false)
    requestAnimationFrame(() => {
      setShaking(true)
      timer.current = setTimeout(() => setShaking(false), duration)
    })
  }, [duration])

  React.useEffect(() => {
    return () => {
      if (timer.current) clearTimeout(timer.current)
    }
  }, [])

  return { shaking, shake }
}
