import { describe, expect, it } from 'bun:test'

import {
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

describe('isStyledComponent', () => {
  it('always reports false', () => {
    expect(isStyledComponent(() => null)).toBe(false)
  })
})
