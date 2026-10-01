'use client'

import { useContext } from 'react'

import { StyledThemeContext } from './theme-context'
import type { StyledTheme } from './theme-vars'

interface CreateThemeOptions {
  /** Prefix of the CSS variable names, `sc` by default */
  prefix?: string
  /** Selector declaring the variables, `:root` by default */
  selector?: string
}

type VarTheme<T> = {
  [K in keyof T]: T[K] extends StyledTheme ? VarTheme<T[K]> : string
}

export type ThemeContract<T extends StyledTheme> = VarTheme<T> & {
  GlobalStyle: () => React.ReactElement
  raw: T
  vars: VarTheme<T>
  resolve: (element?: Element) => T
}

/** Each leaf of `theme` as `leaf(path, value)`, keeping its shape */
function mapLeaves(
  theme: StyledTheme,
  leaf: (path: string, value: string | number) => string | number,
  path = '',
): StyledTheme {
  const result: StyledTheme = {}
  for (const [key, value] of Object.entries(theme)) {
    const next = path ? `${path}-${key}` : key
    result[key] =
      value !== null && typeof value === 'object'
        ? mapLeaves(value, leaf, next)
        : leaf(next, value)
  }
  return result
}

/** The declarations of the leaves `shape` has, with the values `theme` gives */
function declarations(
  shape: StyledTheme,
  theme: StyledTheme | undefined,
  prefix: string,
  path = '',
): string {
  let css = ''
  for (const [key, value] of Object.entries(shape)) {
    const next = path ? `${path}-${key}` : key
    const themed = theme?.[key]
    if (value !== null && typeof value === 'object') {
      if (themed !== null && typeof themed === 'object') {
        css += declarations(value, themed, prefix, next)
      }
    } else if (themed !== undefined && typeof themed !== 'object') {
      css += `--${prefix}${next}:${themed};`
    }
  }
  return css
}

/**
 * styled-components' `createTheme`: the theme with each leaf a
 * `var(--prefix-path, value)` reference, and a `GlobalStyle` declaring the
 * variables from the theme the nearest `ThemeProvider` gives
 */
export function createTheme<T extends StyledTheme>(
  defaultTheme: T,
  options?: CreateThemeOptions,
): ThemeContract<T> {
  const prefix = `${options?.prefix ?? 'sc'}-`
  const selector = options?.selector ?? ':root'
  const refs = mapLeaves(
    defaultTheme,
    (path, value) => `var(--${prefix}${path}, ${value})`,
  )
  function GlobalStyle() {
    const theme = useContext(StyledThemeContext)
    return (
      <style>{`${selector}{${declarations(defaultTheme, theme, prefix)}}`}</style>
    )
  }
  return Object.assign(refs, {
    GlobalStyle,
    raw: defaultTheme,
    vars: mapLeaves(defaultTheme, (path) => `--${prefix}${path}`),
    resolve(element?: Element) {
      const styles = getComputedStyle(element ?? document.documentElement)
      return mapLeaves(
        defaultTheme,
        (path, value) =>
          styles.getPropertyValue(`--${prefix}${path}`).trim() || value,
      )
    },
  }) as unknown as ThemeContract<T>
}
