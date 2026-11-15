import type { CSSProperties } from 'react'

import type { StyledTheme } from '../types/props'

export type { StyledTheme, StyledThemeValue } from '../types/props'

/**
 * Join a theme path into the CSS custom property that carries it, so
 * `theme.colors.brand` and the `--colors-brand` variable always agree.
 */
export function themeVariableName(path: readonly string[]): string {
  return `--${path.join('-')}`
}

/**
 * Flatten a (possibly nested) theme into CSS custom property declarations.
 *
 * `display: contents` keeps the provider out of layout: the element exists only
 * to scope the variables to its subtree, and the cascade handles nesting.
 */
export function themeToCssVariables(theme?: StyledTheme): CSSProperties {
  const style: Record<string, string | number> = { display: 'contents' }
  const walk = (node: StyledTheme, path: string[]) => {
    for (const [key, value] of Object.entries(node)) {
      const next = [...path, key]
      if (value !== null && typeof value === 'object') {
        walk(value, next)
      } else if (value !== undefined) {
        style[themeVariableName(next)] = value
      }
    }
  }
  if (theme) walk(theme, [])
  return style as CSSProperties
}

/**
 * The same variables as one `:root` rule, for what no provider element can
 * reach: global styles on `html`, `body` or `:root` read variables from their
 * own ancestors, never from a descendant element. `<` is escaped so a value
 * cannot close the `<style>` element that carries the rule.
 */
export function themeToRootRule(theme: StyledTheme): string {
  const declarations = Object.entries(themeToCssVariables(theme))
    .filter(([name]) => name.startsWith('--'))
    .map(([name, value]) => `${name}:${String(value).replaceAll('<', '\\3c ')}`)
  return `:root{${declarations.join(';')}}`
}
