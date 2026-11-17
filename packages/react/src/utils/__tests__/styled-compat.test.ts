import { describe, expect, it } from 'bun:test'

import {
  CacheProvider,
  isStyledComponent,
  ServerStyleSheet,
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
  it('recognizes the components the build marks', () => {
    expect(
      isStyledComponent({
        $$typeof: 'forward_ref',
        styledComponentId: 'devup',
      }),
    ).toBe(true)
    expect(
      isStyledComponent(Object.assign(() => null, { styledComponentId: 'x' })),
    ).toBe(true)
  })

  it('rejects anything else', () => {
    expect(isStyledComponent(() => null)).toBe(false)
    expect(isStyledComponent({ $$typeof: 'forward_ref' })).toBe(false)
    expect(isStyledComponent(null)).toBe(false)
    expect(isStyledComponent('div')).toBe(false)
    expect(isStyledComponent(undefined)).toBe(false)
  })
})
