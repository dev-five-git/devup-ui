import { describe, expect, it } from 'bun:test'

import {
  createNodeModulesExcludeRegex,
  type DevupUIBasePluginOptions,
  getFileNumByFilename,
} from '../shared'

describe('getFileNumByFilename', () => {
  it('should return null for devup-ui.css', () => {
    expect(getFileNumByFilename('devup-ui.css')).toBeNull()
  })

  it('should return file number for devup-ui-5.css', () => {
    expect(getFileNumByFilename('devup-ui-5.css')).toBe(5)
  })

  it('should return file number for devup-ui-123.css', () => {
    expect(getFileNumByFilename('devup-ui-123.css')).toBe(123)
  })

  it('should handle query params: devup-ui.css?fileNum=79', () => {
    expect(getFileNumByFilename('devup-ui.css?fileNum=79')).toBe(79)
  })

  it('should handle path with query: /path/to/devup-ui.css?fileNum=42', () => {
    expect(getFileNumByFilename('/path/to/devup-ui.css?fileNum=42')).toBe(42)
  })

  it('should return null for path/to/devup-ui.css (no number, no query)', () => {
    expect(getFileNumByFilename('path/to/devup-ui.css')).toBeNull()
  })

  // Regression: when Next.js `assetPrefix` is set, Turbopack appends extra
  // query parameters (e.g. `?dpl=DEPLOYMENT_ID`) to module URLs. The base
  // CSS file must still be detected correctly, otherwise `@layer b` styles
  // are dropped from the build output.
  it('should return null for devup-ui.css with non-fileNum query (?dpl=...)', () => {
    expect(getFileNumByFilename('devup-ui.css?dpl=abc')).toBeNull()
  })

  it('should return null for devup-ui.css with version query (?v=...)', () => {
    expect(getFileNumByFilename('devup-ui.css?v=12345')).toBeNull()
  })

  it('should return null for path/to/devup-ui.css with deployment query', () => {
    expect(getFileNumByFilename('/path/to/devup-ui.css?dpl=abc')).toBeNull()
  })

  it('should still extract fileNum when extra queries follow it', () => {
    expect(getFileNumByFilename('devup-ui.css?fileNum=5&dpl=abc')).toBe(5)
  })

  it('should still extract fileNum when extra queries precede it', () => {
    expect(getFileNumByFilename('devup-ui.css?dpl=abc&fileNum=7')).toBe(7)
  })

  it('should extract file number from devup-ui-5.css with query', () => {
    expect(getFileNumByFilename('devup-ui-5.css?dpl=abc')).toBe(5)
  })

  it('should return null for unrelated css filename', () => {
    expect(getFileNumByFilename('something-else.css')).toBeNull()
  })
})

describe('createNodeModulesExcludeRegex', () => {
  it('should match node_modules paths that should be excluded', () => {
    const regex = createNodeModulesExcludeRegex([])
    expect(regex.test('node_modules/some-package/index.js')).toBe(true)
    expect(regex.test('/path/to/node_modules/lodash/index.js')).toBe(true)
  })

  it('should NOT match @devup-ui paths', () => {
    const regex = createNodeModulesExcludeRegex([])
    expect(regex.test('node_modules/@devup-ui/react/index.js')).toBe(false)
    expect(regex.test('node_modules/@devup-ui/components/index.js')).toBe(false)
    expect(regex.test('node_modules/@devup-editor/react/index.js')).toBe(false)
  })

  it('should NOT match included packages', () => {
    const regex = createNodeModulesExcludeRegex(['my-company/design-system'])
    expect(regex.test('node_modules/my-company/design-system/index.js')).toBe(
      false,
    )
  })

  it('should handle extra excludes parameter', () => {
    const regex = createNodeModulesExcludeRegex([], '.mdx.[tj]sx?$')
    // Should match node_modules
    expect(regex.test('node_modules/some-package/index.js')).toBe(true)
    // Should also match .mdx.tsx files
    expect(regex.test('src/page.mdx.tsx')).toBe(true)
    expect(regex.test('src/page.mdx.jsx')).toBe(true)
    expect(regex.test('src/page.mdx.ts')).toBe(true)
    // Should NOT match @devup-ui
    expect(regex.test('node_modules/@devup-ui/react/index.js')).toBe(false)
  })

  it('should handle empty include array', () => {
    const regex = createNodeModulesExcludeRegex([])
    expect(regex.test('node_modules/some-package/index.js')).toBe(true)
    expect(regex.test('node_modules/@devup-ui/react/index.js')).toBe(false)
  })
})

describe('DevupUIBasePluginOptions', () => {
  it('should be usable as a type', () => {
    const options: DevupUIBasePluginOptions = {
      package: '@devup-ui/react',
      cssDir: 'df/devup-ui',
      devupFile: 'devup.json',
      distDir: 'df',
      debug: false,
      include: [],
      singleCss: false,
    }
    expect(options.package).toBe('@devup-ui/react')
  })

  it('should accept optional fields', () => {
    const options: DevupUIBasePluginOptions = {
      package: '@devup-ui/react',
      cssDir: 'df/devup-ui',
      devupFile: 'devup.json',
      distDir: 'df',
      debug: false,
      include: [],
      singleCss: false,
      prefix: 'my-prefix',
      importAliases: { '@emotion/styled': 'styled' },
    }
    expect(options.prefix).toBe('my-prefix')
    expect(options.importAliases).toEqual({ '@emotion/styled': 'styled' })
  })
})
