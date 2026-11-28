'use client'

export const STORAGE_KEY = '__DF_THEME_SELECTED__'
const DARK_QUERY = '(prefers-color-scheme:dark)'

export function readSavedTheme(): string | null {
  try {
    return localStorage.getItem(STORAGE_KEY)
  } catch {
    // Blocked storage: no saved theme
    return null
  }
}

export function darkQuery(): MediaQueryList {
  return window.matchMedia(DARK_QUERY)
}

/** The theme the system implies: `dark` when asked to follow a dark system, else the default theme */
export function systemTheme(auto: boolean | undefined): string {
  let dark = false
  if (auto) {
    try {
      dark = darkQuery().matches
    } catch {
      // No media queries: the default theme
    }
  }
  return dark ? 'dark' : (process.env.DEVUP_UI_DEFAULT_THEME ?? 'default')
}
