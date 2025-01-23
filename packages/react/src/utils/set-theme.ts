'use client'

import { DevupTheme } from '../types/theme'

export function setTheme(theme: keyof DevupTheme): void {
  document.documentElement.setAttribute('data-theme', theme)
  localStorage.setItem('__DF_THEME_SELECTED__', theme)
}
