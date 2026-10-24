'use client'

import type { DevupTheme } from '../types/theme'

/** The theme of the page, or null where there is no page to read it from */
export function getTheme():
  (keyof DevupTheme extends undefined ? string : keyof DevupTheme) | null {
  if (typeof document === 'undefined') return null
  return document.documentElement.getAttribute('data-theme') as keyof DevupTheme
}
