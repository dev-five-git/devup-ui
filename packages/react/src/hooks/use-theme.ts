'use client'

import { useSyncExternalStore } from 'react'
import { themeStore } from 'src/stores/themeStore'

export function useTheme() {
  const theme = useSyncExternalStore(
    themeStore.subscribe,
    themeStore.get,
    themeStore.get,
  )
  return theme
}
