'use client'

import type { DevupTheme } from '../types/theme'
import { STORAGE_KEY, systemTheme } from './theme-system'

/**
 * Sets and saves the theme. A saved theme is an explicit choice: it wins over
 * the system preference and reaches other tabs.
 *
 * `null` forgets the choice: the saved theme is removed and the page goes back
 * to the theme it starts with when nothing was chosen, following the system
 * preference again as `ThemeScript` or `initTheme` set it up (other tabs too).
 * Without either running, the page takes the system preference once.
 *
 * Where there is no page (server rendering) nothing is set; a storage that
 * cannot be written only means the choice is not kept.
 */
export function setTheme(
  theme:
    (keyof DevupTheme extends undefined ? string : keyof DevupTheme) | null,
): void {
  if (typeof document === 'undefined') return
  const root = document.documentElement
  if (theme === null) {
    forgetTheme(root)
    return
  }
  root.setAttribute('data-theme', theme)
  try {
    localStorage.setItem(STORAGE_KEY, theme)
  } catch {
    // Blocked storage: the theme still applies to this page
  }
}

function forgetTheme(root: HTMLElement) {
  try {
    localStorage.removeItem(STORAGE_KEY)
  } catch {
    // Blocked storage: nothing was kept
  }
  root.removeAttribute('data-theme')
  // `ThemeScript` and `initTheme` listen for the theme storage changing: they
  // re-apply the first-load theme with their own `auto` and default
  try {
    window.dispatchEvent(new StorageEvent('storage', { key: STORAGE_KEY }))
  } catch {
    // No storage events: the fallback below applies
  }
  if (!root.hasAttribute('data-theme'))
    root.setAttribute('data-theme', systemTheme(true))
}
