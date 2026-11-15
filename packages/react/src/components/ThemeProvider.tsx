'use client'

import type { ReactNode } from 'react'
import { useContext } from 'react'

import { StyledThemeContext } from '../utils/theme-context'
import type { StyledTheme } from '../utils/theme-vars'
import { themeToCssVariables, themeToRootRule } from '../utils/theme-vars'

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
 * The variables scope to the subtree on a `display: contents` element. The
 * outermost provider also declares them on `:root`, so global styles on
 * `html`, `body` or `:root` (`createGlobalStyle`, `<Global>`) read the theme
 * too. That rule is an ordinary `<style>` element, so server rendering sends
 * it with the markup, a theme change updates it and unmounting removes it.
 */
export function ThemeProvider({ theme, children }: ThemeProviderProps) {
  const outer = useContext(StyledThemeContext)
  const resolved = resolveTheme(theme, outer)
  return (
    <StyledThemeContext.Provider value={resolved}>
      {outer === undefined && resolved !== undefined && (
        <style
          dangerouslySetInnerHTML={{ __html: themeToRootRule(resolved) }}
        />
      )}
      <div style={themeToCssVariables(resolved)}>{children}</div>
    </StyledThemeContext.Provider>
  )
}
