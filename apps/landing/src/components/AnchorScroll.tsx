'use client'

import { useEffect } from 'react'

export function AnchorScroll() {
  useEffect(() => {
    let cancelled = false

    async function scrollToAnchor() {
      if (!window.location.hash) return

      await document.fonts?.ready
      await new Promise<void>((resolve) => {
        requestAnimationFrame(() => requestAnimationFrame(() => resolve()))
      })

      if (cancelled) return

      const target = document.getElementById(
        decodeURIComponent(window.location.hash.slice(1)),
      )
      target?.scrollIntoView({ block: 'start' })
    }

    void scrollToAnchor()
    window.addEventListener('hashchange', scrollToAnchor)

    return () => {
      cancelled = true
      window.removeEventListener('hashchange', scrollToAnchor)
    }
  }, [])

  return null
}
