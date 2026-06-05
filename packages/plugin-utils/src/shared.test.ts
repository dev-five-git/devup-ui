import { describe, expect, it } from 'bun:test'

import { getFileNumByFilename } from './shared'

describe('getFileNumByFilename', () => {
  it('parses the file number from a standard filename', () => {
    expect(getFileNumByFilename('devup-ui-5.css')).toBe(5)
    expect(getFileNumByFilename('devup-ui-0.css')).toBe(0)
    expect(getFileNumByFilename('devup-ui-123.css')).toBe(123)
  })

  it('returns null for the base devup-ui.css', () => {
    expect(getFileNumByFilename('devup-ui.css')).toBeNull()
    expect(getFileNumByFilename('df/devup-ui/devup-ui.css')).toBeNull()
  })

  it('parses the Turbopack query-parameter format', () => {
    expect(getFileNumByFilename('devup-ui.css?fileNum=79')).toBe(79)
  })

  it('strips trailing queries (e.g. Next assetPrefix ?dpl=...) before matching', () => {
    expect(getFileNumByFilename('devup-ui.css?dpl=DEPLOYMENT_ID')).toBeNull()
    expect(getFileNumByFilename('devup-ui-7.css?dpl=DEPLOYMENT_ID')).toBe(7)
  })

  it('returns null for unrelated css files', () => {
    expect(getFileNumByFilename('styles.css')).toBeNull()
    expect(getFileNumByFilename('foo/bar.css')).toBeNull()
  })

  // Regression: the filename must be parsed from the BASENAME, not by splitting
  // the whole path on "devup-ui-". A project path/dir that itself contains
  // "devup-ui-" (e.g. a folder named `next-devup-ui-collapse`, or the cssDir
  // `.next/cache/devup-ui_<id>` sitting under such a folder) previously caused
  // split('devup-ui-')[1] to grab the WRONG segment -> NaN -> null, so the
  // css-loader served the base sheet instead of the bucket. Webpack uses the
  // `devup-ui-N.css` filename form, so this silently dropped collapsed-bucket
  // atoms; Turbopack was immune because it uses the `?fileNum=` query form.
  it('parses the basename even when an ancestor directory contains "devup-ui-"', () => {
    expect(
      getFileNumByFilename(
        'C:\\repo\\next-devup-ui-collapse\\.next\\cache\\devup-ui_DML7Ct3\\devup-ui-0.css',
      ),
    ).toBe(0)
    expect(
      getFileNumByFilename(
        '/repo/my-devup-ui-app/.next/cache/devup-ui_abc/devup-ui-12.css',
      ),
    ).toBe(12)
    // base file inside a "devup-ui-" ancestor must still resolve to null
    expect(
      getFileNumByFilename(
        '/repo/next-devup-ui-collapse/df/devup-ui/devup-ui.css',
      ),
    ).toBeNull()
  })
})
