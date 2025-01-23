'use client'
import { useId, useState } from 'react'

import type { DevupTheme } from '../types/theme'
import { useSafeEffect } from './use-safe-effect'

let observer: null | MutationObserver = null
const setThemeMap: Record<string, React.Dispatch<keyof DevupTheme>> = {}
let globalTheme: keyof DevupTheme | null = null

export function useTheme(): keyof DevupTheme | null {
  const id = useId()
  const [theme, setTheme] = useState<keyof DevupTheme | null>(globalTheme)
  useSafeEffect(() => {
    if (globalTheme !== null) return
    const currentTheme = document.documentElement.getAttribute('data-theme')
    if (currentTheme !== null && currentTheme !== theme)
      setTheme(currentTheme as keyof DevupTheme)
  }, [])
  useSafeEffect(() => {
    const targetNode = document.documentElement
    setThemeMap[id] = setTheme
    if (!observer) {
      observer = new MutationObserver(() => {
        const theme = document.documentElement.getAttribute('data-theme')
        globalTheme = theme as keyof DevupTheme
        for (const key in setThemeMap)
          setThemeMap[key](theme as keyof DevupTheme)
      })
      observer.observe(targetNode, {
        attributes: true,
        attributeFilter: ['data-theme'],
        childList: false,
        subtree: false,
        characterData: false,
        attributeOldValue: false,
        characterDataOldValue: false,
      })
    }

    return () => {
      delete setThemeMap[id]
      if (observer && Object.keys(setThemeMap).length === 0)
        observer.disconnect()
    }
  }, [id])
  return theme
}
