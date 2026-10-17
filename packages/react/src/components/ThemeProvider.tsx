'use client'

import type { ReactNode } from 'react'
import { useContext } from 'react'

import { StyledThemeContext } from '../utils/theme-context'
import type { StyledTheme } from '../utils/theme-vars'
import { themeToCssVariables } from '../utils/theme-vars'

export type ThemeArgument =
  StyledTheme | ((outer: StyledTheme | undefined) => StyledTheme)

interface ThemeProviderProps {
  theme?: ThemeArgument
  children?: ReactNode
}

/**
 * The theme a provider gives: a function receives the outer theme and
 * returns the new one, an object merges over the outer theme, as
 * styled-components and Emotion resolve nested providers
 */
export function resolveTheme(
  theme: ThemeArgument | undefined,
  outer: StyledTheme | undefined,
): StyledTheme | undefined {
  if (typeof theme === 'function') return theme(outer)
  if (!theme) return outer
  return outer ? { ...outer, ...theme } : theme
}

export function ThemeProvider({ theme, children }: ThemeProviderProps) {
  const resolved = resolveTheme(theme, useContext(StyledThemeContext))
  return (
    <StyledThemeContext.Provider value={resolved}>
      <div style={themeToCssVariables(resolved)}>{children}</div>
    </StyledThemeContext.Provider>
  )
}
