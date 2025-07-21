'use client'

import type { DevupTheme } from '../types/theme'

export function setTheme(
  theme: keyof DevupTheme extends undefined ? string : keyof DevupTheme,
): void {
  document.documentElement.setAttribute('data-theme', theme)
  localStorage.setItem('__DF_THEME_SELECTED__', theme)
}
