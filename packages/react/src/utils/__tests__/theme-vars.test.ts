import { describe, expect, it } from 'bun:test'

import type { StyledTheme } from '../theme-vars'
import {
  themeToCssVariables,
  themeVariableName,
  themeVariables,
} from '../theme-vars'

describe('themeVariableName', () => {
  it('joins a path into a custom property', () => {
    expect(themeVariableName(['colors', 'brand'])).toBe('--colors-brand')
    expect(themeVariableName(['brand'])).toBe('--brand')
  })
})

describe('themeVariables', () => {
  it('declares nothing without a theme', () => {
    expect(themeVariables()).toEqual({})
  })

  it('flattens nested themes and arrays, skipping null and undefined', () => {
    const theme = {
      brand: 'red',
      size: 4,
      space: [0, 4] as unknown as StyledTheme,
      colors: { accent: 'blue', nested: { deep: 'green' } },
      missing: undefined as never,
      empty: null as never,
      'ㄱ_-': 'x',
    }
    expect(themeVariables(theme)).toEqual({
      '--brand': 'red',
      '--size': 4,
      '--space-0': 0,
      '--space-1': 4,
      '--colors-accent': 'blue',
      '--colors-nested-deep': 'green',
      '--ㄱ_-': 'x',
    })
    expect(themeVariables(theme)).toBe(themeVariables(theme))
  })

  it('rejects two paths that give the same variable', () => {
    expect(() => themeVariables({ a: { b: 'red' }, 'a-b': 'blue' })).toThrow(
      'Cannot use theme path `a-b`: it gives the same CSS variable `--a-b` as `a.b`. Fix: rename one of the keys so each path joins to its own name',
    )
  })

  it('rejects a theme that contains itself', () => {
    const theme: StyledTheme = { colors: { brand: 'red' } }
    ;(theme.colors as StyledTheme).self = theme
    expect(() => themeVariables(theme)).toThrow(
      'Cannot use theme path `colors.self`: the theme contains itself. Fix: remove the reference back to an enclosing object',
    )
  })

  it('accepts one object reached through two paths', () => {
    const shared = { x: 1 }
    expect(themeVariables({ a: shared, b: shared })).toEqual({
      '--a-x': 1,
      '--b-x': 1,
    })
  })

  it('rejects a key that cannot be part of a variable name', () => {
    expect(() => themeVariables({ colors: { 'brand color': 'red' } })).toThrow(
      'Cannot use theme path `colors.brand color`: the key `brand color` cannot be part of a CSS variable name. Fix: use keys made of letters, digits, `-` and `_`',
    )
    expect(() => themeVariables({ '': 'red' })).toThrow('(root)')
  })
})

describe('themeToCssVariables', () => {
  it('keeps the provider out of layout', () => {
    expect(themeToCssVariables()).toEqual({ display: 'contents' })
  })

  it('declares every variable without an outer theme', () => {
    expect(
      themeToCssVariables({
        brand: 'red',
        colors: { accent: 'blue' },
      }) as Record<string, unknown>,
    ).toEqual({
      display: 'contents',
      '--brand': 'red',
      '--colors-accent': 'blue',
    })
  })

  it('declares only what differs from the outer theme', () => {
    expect(
      themeToCssVariables(
        { brand: 'red', accent: 'green', extra: 1 },
        { brand: 'red', accent: 'blue' },
      ) as Record<string, unknown>,
    ).toEqual({ display: 'contents', '--accent': 'green', '--extra': 1 })
  })
})
