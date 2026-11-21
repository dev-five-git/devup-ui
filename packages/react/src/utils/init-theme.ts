'use client'

import type { DevupTheme } from '../types/theme'
import type { Conditional } from '../types/utils'
import {
  darkQuery,
  readSavedTheme,
  STORAGE_KEY,
  systemTheme,
} from './theme-system'

/**
 * Initialize the theme, if you can't use the `ThemeScript` component
 * e.g. in vite
 *
 * A theme saved by `setTheme` is an explicit choice and wins. Without one, the
 * system preference decides when `auto` is set, and is followed as it changes;
 * a theme saved in another tab is followed too, and so is `setTheme(null)`
 * forgetting the choice. Where there is no page (server rendering) nothing
 * happens.
 * @param auto - Whether to use the system theme
 * @param theme - The theme to use
 * @returns Stops following the system and other tabs
 */
export function initTheme(
  auto?: boolean,
  theme?: Conditional<DevupTheme>,
): () => void {
  if (typeof document === 'undefined') return () => {}
  const root = document.documentElement
  if (theme) {
    root.setAttribute('data-theme', theme)
    return () => {}
  }

  const apply = () => {
    root.setAttribute('data-theme', readSavedTheme() || systemTheme(auto))
  }
  const onStorage = (e: StorageEvent) => {
    if (e.key === STORAGE_KEY || e.key === null) apply()
  }
  apply()
  window.addEventListener('storage', onStorage)
  let query: MediaQueryList | undefined
  if (auto) {
    try {
      query = darkQuery()
      query.addEventListener('change', apply)
    } catch {
      query = undefined
    }
  }
  return () => {
    window.removeEventListener('storage', onStorage)
    query?.removeEventListener('change', apply)
  }
}
