import { describe, expect, it } from 'bun:test'

import {
  CacheProvider,
  isStyledComponent,
  ServerStyleSheet,
  StyleSheetInstance,
  StyleSheetManager,
} from '../styled-compat'

describe('ServerStyleSheet', () => {
  it('collects nothing because the stylesheet is already on disk', () => {
    const sheet = new ServerStyleSheet()
    expect(sheet.collectStyles('children')).toBe('children')
    expect(sheet.getStyleTags()).toBe('')
    expect(sheet.getStyleElement()).toEqual([])
    expect(sheet.seal()).toBeUndefined()
  })

  it('gives an empty instance SSR setups pass on and clear', () => {
    const sheet = new ServerStyleSheet()
    expect(sheet.instance).toBeInstanceOf(StyleSheetInstance)
    expect(sheet.instance.clearTag()).toBeUndefined()
    expect(String(sheet.instance)).toBe('')
    expect(
      StyleSheetManager({ sheet: sheet.instance, children: 'child' }),
    ).toBe('child')
  })
})

describe('StyleSheetManager', () => {
  it('renders its children untouched', () => {
    expect(StyleSheetManager({ children: 'child' })).toBe('child')
    expect(StyleSheetManager({})).toBeUndefined()
  })
})

describe('CacheProvider', () => {
  it('renders its children untouched', () => {
    expect(CacheProvider({ value: {}, children: 'child' })).toBe('child')
    expect(CacheProvider({})).toBeUndefined()
  })
})

describe('isStyledComponent', () => {
  it('always reports false', () => {
    expect(isStyledComponent(() => null)).toBe(false)
  })
})
