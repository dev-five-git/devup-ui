import { describe, expect, it } from 'bun:test'

import { transformStaticVanillaExtract } from '../static-vanilla'

describe('transformStaticVanillaExtract', () => {
  it('converts static style modules without evaluating JavaScript', () => {
    const source = `import { style } from '@vanilla-extract/css'
export const box = style({ color: 'red', padding: [4, null, 8] })
export const hover = style({ selectors: { '&:hover': { opacity: 0.5 } } })`

    expect(
      transformStaticVanillaExtract(
        'src/styles.css.ts',
        source,
        '@devup-ui/react',
      ),
    ).toBe(`import { css } from '@devup-ui/react'
export const box = css({ color: 'red', padding: [4, null, 8] })
export const hover = css({ selectors: { '&:hover': { opacity: 0.5 } } })`)
  })

  it('rejects executable values and extra module statements', () => {
    const dynamic = `import { style } from '@vanilla-extract/css'
const color = getColor()
export const box = style({ color })`
    const spread = `import { style } from '@vanilla-extract/css'
export const box = style({ ...base, color: 'red' })`

    expect(
      transformStaticVanillaExtract(
        'styles.css.ts',
        dynamic,
        '@devup-ui/react',
      ),
    ).toBeUndefined()
    expect(
      transformStaticVanillaExtract('styles.css.ts', spread, '@devup-ui/react'),
    ).toBeUndefined()
    expect(
      transformStaticVanillaExtract('styles.ts', dynamic, '@devup-ui/react'),
    ).toBeUndefined()
  })

  it('preserves comments and aliases in the static subset', () => {
    const source = `/* import */ import { style as makeStyle } from '@vanilla-extract/css'
export const box = makeStyle({
  // responsive values remain source text
  padding: [4, null, 8],
})`

    expect(
      transformStaticVanillaExtract('styles.css.js', source, 'custom-package'),
    ).toContain('export const box = css({')
  })

  it('rejects unsupported imports and malformed trailing comments', () => {
    const multipleImports = `import { style, globalStyle } from '@vanilla-extract/css'
export const box = style({ color: 'red' })`
    const unterminatedComment = `import { style } from '@vanilla-extract/css'
export const box = style({ color: 'red' }) /*`

    expect(
      transformStaticVanillaExtract(
        'styles.css.ts',
        multipleImports,
        '@devup-ui/react',
      ),
    ).toBeUndefined()
    expect(
      transformStaticVanillaExtract(
        'styles.css.ts',
        unterminatedComment,
        '@devup-ui/react',
      ),
    ).toBeUndefined()
  })
})
