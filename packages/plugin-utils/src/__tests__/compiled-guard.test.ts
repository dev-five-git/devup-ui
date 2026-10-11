import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import {
  createCompileTimeClassifier,
  isCompileTimeAlias,
  UntransformedSourceError,
} from '../compiled-guard'
import { createDependencyGuard } from '../dependency-guard'
import {
  isMdxSource,
  mdxSourceFilter,
  normalizeMdxExtensions,
  selectedSourceFilter,
} from '../mdx-selection'

it('normalizes literal selection and preserves exact source filtering', () => {
  const extensions = normalizeMdxExtensions(['.MDOWN', '.mdx', '.mdown'])
  expect(extensions).toEqual(['.mdown', '.mdx'])
  expect(isMdxSource('page.MDOWN?query', extensions)).toBe(true)
  expect(mdxSourceFilter(extensions).test('page.md')).toBe(false)
  expect(selectedSourceFilter(extensions).test('page.mts')).toBe(true)
  expect(mdxSourceFilter([]).test('page.mdx')).toBe(false)
  expect(normalizeMdxExtensions()).toEqual(['.mdx'])
  expect(() => normalizeMdxExtensions(['*.mdx'])).toThrow('mdxExtensions:1:1')
})

it('offers alias disabling only for an active compile-time compatibility alias', () => {
  const reference = {
    request: '@devup-ui/react',
    ids: ['Box'],
    line: 1,
    column: 1,
  }
  expect(
    new UntransformedSourceError('/page.mdown', reference).message,
  ).not.toContain('importAliases')
  expect(
    new UntransformedSourceError(
      '/page.mdown',
      { ...reference, request: '@emotion/styled' },
      true,
    ).message,
  ).toContain('importAliases["@emotion/styled"]=false')
})

it('classifies only actual public surfaces instead of names or directory membership', () => {
  const root = mkdtempSync(join(tmpdir(), 'devup-surface-'))
  mkdirSync(join(root, 'dist/compat'), { recursive: true })
  writeFileSync(
    join(root, 'package.json'),
    JSON.stringify({
      name: '@devup-ui/react',
      exports: {
        '.': { types: './dist/index.d.ts', import: './dist/index.js' },
        './compat': './dist/compat/index.js',
        './stylex': './dist/stylex.cjs',
        './types': {},
      },
    }),
  )
  try {
    const classify = createCompileTimeClassifier('@devup-ui/react')
    expect(classify(join(root, 'dist/index.js'), ['Box'])).toBe(true)
    expect(classify(join(root, 'dist/index.js'), ['getTheme'])).toBe(false)
    expect(classify(join(root, 'dist/index.js'), ['stylex', 'create'])).toBe(
      true,
    )
    expect(classify(join(root, 'dist/index.js'), ['stylex'])).toBe(false)
    expect(classify(join(root, 'dist/compat/index.js'), ['Global'])).toBe(true)
    expect(
      classify(join(root, 'dist/compat/index.js'), ['ThemeProvider']),
    ).toBe(false)
    expect(classify(join(root, 'dist/stylex.cjs'), ['props'])).toBe(true)
    expect(classify(join(root, 'dist/ordinary.js'), ['Box'])).toBe(false)
    expect(classify(join(tmpdir(), 'Box.js'), ['Box'])).toBe(false)
    mkdirSync(join(root, 'stylex'))
    writeFileSync(
      join(root, 'stylex/package.json'),
      JSON.stringify({
        name: '@stylexjs/stylex',
        exports: { '.': './index.js' },
      }),
    )
    expect(classify(join(root, 'stylex/index.js'), ['default', 'create'])).toBe(
      true,
    )
    expect(classify(join(root, 'stylex/index.js'), ['default'])).toBe(false)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

it.each([
  ['@emotion/styled', ['default'], { '@emotion/styled': 'styled' }, true],
  ['@emotion/react', ['CacheProvider'], { '@emotion/react': null }, false],
  [
    'styled-components',
    ['ThemeProvider'],
    { 'styled-components': 'styled' },
    false,
  ],
  ['@vanilla-extract/css', ['style'], { '@vanilla-extract/css': null }, true],
  ['custom', ['Box'], { custom: null }, true],
  ['custom', [], { custom: 'styled' }, false],
  ['custom', ['default'], { custom: null }, false],
  ['disabled', ['css'], {}, false],
  ['@stylexjs/stylex', ['default', 'create'], {}, false],
])(
  'uses extractor-compatible alias mapping for %s/%s',
  (request, ids, aliases, expected) => {
    expect(
      isCompileTimeAlias({ request, ids, line: 1, column: 1 }, aliases),
    ).toBe(expected)
  },
)

it('guards used dependency IDs without guarding unused imports, opaque namespaces or ordinary JS', () => {
  const check = createDependencyGuard({
    package: '@devup-ui/react',
    mdxExtensions: ['.mdx'],
    importAliases: { alias: 'styled' },
  })
  const dependency = {
    type: 'esm import specifier',
    request: 'alias',
    ids: ['default'],
    loc: { start: { line: 3, column: 4 } },
  }
  const modules = [
    {
      type: 'javascript/auto',
      resource: '/page.mdown',
      dependencies: [
        dependency,
        { ...dependency, ids: [] },
        { ...dependency, type: 'esm import' },
        null,
      ],
    },
    {
      type: 'javascript/auto',
      resource: '/node_modules/skipped.js',
      dependencies: [dependency],
    },
    {
      type: 'asset/source',
      resource: '/asset.blob',
      dependencies: [dependency],
    },
    { type: 'javascript/auto', dependencies: [dependency] },
  ]
  const errors = check(modules, { graph: {}, target: () => undefined })
  expect(errors).toHaveLength(1)
  expect(errors[0]?.message).toContain('/page.mdown:3:5:')
})

it('consumes public getIds and require-member names but ignores untyped or opaque IDs', () => {
  const check = createDependencyGuard({
    package: '@devup-ui/react',
    mdxExtensions: [],
    importAliases: { alias: null },
  })
  const module = {
    type: 'javascript/auto',
    resource: '/page.unknown',
    dependencies: [
      {
        type: 'harmony import specifier',
        request: 'alias',
        getIds: () => ['css'],
      },
      { type: 'cjs full require', request: 'alias', names: ['css'] },
      { type: 'cjs full require', request: 'alias', names: [123] },
    ],
  }
  expect(
    check([module], {
      graph: {},
      target: () => undefined,
      forwarded: () => false,
    }),
  ).toHaveLength(2)
})
