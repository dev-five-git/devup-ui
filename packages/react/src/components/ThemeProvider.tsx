'use client'

import type { ReactNode } from 'react'
import { useContext, useMemo } from 'react'

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

/**
 * Gives `theme` to `useTheme`, `withTheme` and `ThemeConsumer` through
 * context, and declares the CSS variables styles read on a `display:
 * contents` element. CSS variables need an element to live on, so a
 * provider given a `theme` renders one, declaring only the variables that
 * differ from the outer provider's; a provider without one adds no element.
 * Whether the element exists follows only the `theme` prop being given, so a
 * changing theme never remounts the subtree.
 */
export function ThemeProvider({ theme, children }: ThemeProviderProps) {
  const outer = useContext(StyledThemeContext)
  const resolved = useMemo(() => resolveTheme(theme, outer), [theme, outer])
  const style = useMemo(
    () => themeToCssVariables(resolved, outer),
    [resolved, outer],
  )
  return (
    <StyledThemeContext.Provider value={resolved}>
      {theme === undefined ? children : <div style={style}>{children}</div>}
    </StyledThemeContext.Provider>
  )
}
