import { describe, expect, it } from 'bun:test'

import {
  createThemeAccessor,
  themeToCssVariables,
  themeVariableName,
} from '../theme-vars'

describe('themeVariableName', () => {
  it('joins a path into a custom property', () => {
    expect(themeVariableName(['colors', 'brand'])).toBe('--colors-brand')
    expect(themeVariableName(['brand'])).toBe('--brand')
  })
})

describe('themeToCssVariables', () => {
  it('keeps the provider out of layout', () => {
    expect(themeToCssVariables()).toEqual({ display: 'contents' })
  })

  it('flattens nested themes and skips undefined values', () => {
    const style = themeToCssVariables({
      brand: 'red',
      size: 4,
      colors: { accent: 'blue', nested: { deep: 'green' } },
      missing: undefined as never,
    })
    expect(style as Record<string, unknown>).toEqual({
      display: 'contents',
      '--brand': 'red',
      '--size': 4,
      '--colors-accent': 'blue',
      '--colors-nested-deep': 'green',
    })
  })

  it('treats null as a value rather than a nested theme', () => {
    const style = themeToCssVariables({
      brand: null as never,
    })
    expect(style as Record<string, unknown>).toEqual({
      display: 'contents',
      '--brand': null,
    })
  })
})

describe('createThemeAccessor', () => {
  it('resolves reads to css variable references', () => {
    const theme = createThemeAccessor<{ colors: { brand: string } }>()
    expect(`${theme.colors.brand}`).toBe('var(--colors-brand)')
    expect(String(theme.colors.brand)).toBe('var(--colors-brand)')
    expect(theme.colors.brand.valueOf()).toBe('var(--colors-brand)')
    expect(theme.colors.brand.toString()).toBe('var(--colors-brand)')
  })

  it('has no reference at the root and ignores symbol keys', () => {
    const theme = createThemeAccessor()
    expect(`${theme}`).toBe('')
    expect(
      (theme as unknown as Record<symbol, unknown>)[Symbol.iterator],
    ).toBeUndefined()
  })
})
