'use client'

import { DevupTheme } from '../types/theme'

export function getTheme(): keyof DevupTheme | null {
  return document.documentElement.getAttribute('data-theme') as keyof DevupTheme
}
