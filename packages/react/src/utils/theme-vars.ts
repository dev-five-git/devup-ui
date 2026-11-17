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
 * A key that joins into a custom property name as it is: letters, digits,
 * `-`, `_` and non-ASCII characters, as the build maps `theme.a.b` and
 * `theme.space[2]` to `--a-b` and `--space-2`
 */
const SEGMENT = /^[\w\u0080-￿-]+$/

const cache = new WeakMap<StyledTheme, Record<string, string | number>>()

function themeError(path: readonly string[], cause: string, fix: string) {
  return new Error(
    `Cannot use theme path \`${path.join('.') || '(root)'}\`: ${cause}. Fix: ${fix}`,
  )
}

function walk(
  node: StyledTheme,
  path: string[],
  variables: Record<string, string | number>,
  owners: Map<string, string>,
  ancestors: Set<object>,
) {
  ancestors.add(node)
  for (const [key, value] of Object.entries(node)) {
    const next = [...path, key]
    if (!SEGMENT.test(key)) {
      throw themeError(
        next,
        `the key \`${key}\` cannot be part of a CSS variable name`,
        'use keys made of letters, digits, `-` and `_`',
      )
    }
    // `null` and `undefined` leaves declare nothing, so the variable keeps
    // what an outer provider (or the stylesheet) gives it
    if (value === null || value === undefined) continue
    if (typeof value === 'object') {
      if (ancestors.has(value)) {
        throw themeError(
          next,
          'the theme contains itself',
          'remove the reference back to an enclosing object',
        )
      }
      walk(value, next, variables, owners, ancestors)
      continue
    }
    const name = themeVariableName(next)
    const owner = owners.get(name)
    if (owner !== undefined) {
      throw themeError(
        next,
        `it gives the same CSS variable \`${name}\` as \`${owner}\``,
        'rename one of the keys so each path joins to its own name',
      )
    }
    owners.set(name, next.join('.'))
    variables[name] = value
  }
  ancestors.delete(node)
}

/**
 * The CSS custom properties a (possibly nested) theme declares, checked so
 * each path owns one variable. Arrays keep their indexes as segments.
 * Throws on a cycle, two paths sharing a variable, or a key that cannot be
 * part of a variable name, instead of silently dropping values.
 */
export function themeVariables(
  theme?: StyledTheme,
): Readonly<Record<string, string | number>> {
  if (!theme) return {}
  const cached = cache.get(theme)
  if (cached) return cached
  const variables: Record<string, string | number> = {}
  walk(theme, [], variables, new Map(), new Set())
  cache.set(theme, variables)
  return variables
}

/**
 * The declarations a provider needs on its scoping element: the variables of
 * `theme` whose values differ from what `outer` already declares, as the
 * cascade gives the rest.
 *
 * `display: contents` keeps that element out of layout; it exists only to
 * scope the variables to its subtree.
 */
export function themeToCssVariables(
  theme?: StyledTheme,
  outer?: StyledTheme,
): CSSProperties {
  const inherited = themeVariables(outer)
  const style: Record<string, string | number> = { display: 'contents' }
  for (const [name, value] of Object.entries(themeVariables(theme))) {
    if (inherited[name] !== value) style[name] = value
  }
  return style as CSSProperties
}
