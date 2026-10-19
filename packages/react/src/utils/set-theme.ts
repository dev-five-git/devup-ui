'use client'

import type { DevupTheme } from '../types/theme'

/**
 * Sets and saves the theme. A saved theme is an explicit choice: it wins over
 * the system preference and reaches other tabs. Where there is no page (server
 * rendering) nothing is set; a storage that cannot be written only means the
 * choice is not kept.
 */
export function setTheme(
  theme: keyof DevupTheme extends undefined ? string : keyof DevupTheme,
): void {
  if (typeof document === 'undefined') return
  document.documentElement.setAttribute('data-theme', theme)
  try {
    localStorage.setItem('__DF_THEME_SELECTED__', theme)
  } catch {
    // Blocked storage: the theme still applies to this page
  }
}
