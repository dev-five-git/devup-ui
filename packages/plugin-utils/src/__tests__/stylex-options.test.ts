import { describe, expect, it } from 'bun:test'

import {
  normalizeStylexOptions,
  STYLEX_STYLE_RESOLUTIONS,
  StylexConfigError,
} from '../index'

describe('normalizeStylexOptions', () => {
  it('defaults to property-specificity', () => {
    expect(normalizeStylexOptions(undefined)).toBe('property-specificity')
    expect(normalizeStylexOptions({})).toBe('property-specificity')
    expect(normalizeStylexOptions({ styleResolution: undefined })).toBe(
      'property-specificity',
    )
  })

  it.each(STYLEX_STYLE_RESOLUTIONS)('accepts %s', (styleResolution) => {
    expect(normalizeStylexOptions({ styleResolution })).toBe(styleResolution)
  })

  it.each([
    ['last-wins', 'got "last-wins"'],
    ['Application-Order', 'got "Application-Order"'],
    [null, 'got null'],
    [1, 'got number'],
  ])('rejects the styleResolution %p', (styleResolution, got) => {
    const error = (() => {
      try {
        normalizeStylexOptions({ styleResolution })
      } catch (error) {
        return error
      }
    })()
    expect(error).toBeInstanceOf(StylexConfigError)
    expect(error).toMatchObject({
      name: 'StylexConfigError',
      path: 'stylex.styleResolution',
      value: styleResolution,
      message: `devup-ui option stylex.styleResolution must be one of 'property-specificity', 'application-order', 'legacy-expand-shorthands', ${got}`,
    })
  })

  it.each([null, 'application-order', ['application-order']])(
    'rejects a stylex option that is not an object: %p',
    (stylex) => {
      expect(() => normalizeStylexOptions(stylex)).toThrow(
        /^devup-ui option stylex must be an object, got /,
      )
    },
  )
})
