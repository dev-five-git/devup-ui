'use client'

import type { DevupTheme } from '../types/theme'
import type { Conditional } from '../types/utils'

const STORAGE_KEY = '__DF_THEME_SELECTED__'
const DARK_QUERY = '(prefers-color-scheme:dark)'

function readSavedTheme(): string | null {
  try {
    return localStorage.getItem(STORAGE_KEY)
  } catch {
    // Blocked storage: no saved theme
    return null
  }
}

function prefersDark(): boolean {
  try {
    return window.matchMedia(DARK_QUERY).matches
  } catch {
    return false
  }
}

/**
 * Initialize the theme, if you can't use the `ThemeScript` component
 * e.g. in vite
 *
 * A theme saved by `setTheme` is an explicit choice and wins. Without one, the
 * system preference decides when `auto` is set, and is followed as it changes;
 * a theme saved in another tab is followed too. Where there is no page (server
 * rendering) nothing happens.
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
    root.setAttribute(
      'data-theme',
      readSavedTheme() ||
        (auto && prefersDark()
          ? 'dark'
          : (process.env.DEVUP_UI_DEFAULT_THEME ?? 'default')),
    )
  }
  const onStorage = (e: StorageEvent) => {
    if (e.key === STORAGE_KEY || e.key === null) apply()
  }
  apply()
  window.addEventListener('storage', onStorage)
  let query: MediaQueryList | undefined
  if (auto) {
    try {
      query = window.matchMedia(DARK_QUERY)
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
