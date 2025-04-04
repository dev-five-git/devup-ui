'use client'

import type { Conditional } from 'src/types/utils'

import { DevupTheme } from '../types/theme'

/**
 * Initialize the theme, if you can't use the `ThemeScript` component
 * e.g. in vite
 * @param auto - Whether to use the system theme
 * @param theme - The theme to use
 */
export function initTheme(
  auto?: boolean,
  theme?: Conditional<DevupTheme>,
): void {
  if (theme) {
    document.documentElement.setAttribute('data-theme', theme)
  } else {
    document.documentElement.setAttribute(
      'data-theme',
      localStorage.getItem('__DF_THEME_SELECTED__') ||
        (auto && window.matchMedia('(prefers-color-scheme:dark)').matches
          ? 'dark'
          : (process.env.DEVUP_UI_DEFAULT_THEME ?? 'default')),
    )
  }
}
