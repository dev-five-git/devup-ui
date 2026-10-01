'use client'

import { useContext } from 'react'

import { StyledThemeContext } from '../utils/theme-context'
import type { StyledTheme } from '../utils/theme-vars'

/**
 * styled-components and Emotion compatible `useTheme`: the theme object the
 * nearest `ThemeProvider` gives, or an empty object without one.
 *
 * Distinct from devup-ui's own `useTheme`, which reports the active theme name.
 */
export function useStyledTheme<T extends StyledTheme = StyledTheme>(): T {
  return (useContext(StyledThemeContext) ?? {}) as T
}
