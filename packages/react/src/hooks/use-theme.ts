'use client'

import { useSyncExternalStore } from 'react'

import { createThemeStore } from '../stores/theme-store'

const themeStore = createThemeStore()

export function useTheme() {
  const theme = useSyncExternalStore(
    themeStore.subscribe,
    themeStore.get,
    themeStore.get,
  )
  return theme
}
