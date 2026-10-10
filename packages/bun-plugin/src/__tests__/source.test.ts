import { join } from 'node:path'

import { expect, it, spyOn } from 'bun:test'

import {
  importsCompiledPackage,
  mentionsCompiledPackage,
  preserveDependencies,
  runtimeSourceFilter,
  sourceLoader,
} from '../source'

it.each(['ts', 'mts', 'cts', 'tsx', 'js', 'mjs', 'cjs', 'jsx'])(
  'chooses the Bun loader when the source extension is %s',
  (extension) => {
    const filename = `source.${extension.toUpperCase()}`
    const expected = extension.endsWith('tsx')
      ? 'tsx'
      : extension.endsWith('ts')
        ? 'ts'
        : extension === 'jsx'
          ? 'jsx'
          : 'js'
    const loader = sourceLoader(filename)
    expect(loader).toBe(expected)
  },
)

it.each(['@devup-ui/react', '@emotion/react/jsx-runtime', '@stylexjs/stylex'])(
  'recognizes a real import when its package is %s',
  (name) => {
    const source = `import * as styles from '${name}'; export { styles }`
    const found = importsCompiledPackage(source, 'js')
    expect(found).toBe(true)
  },
)

it.each(['js', 'jsx', 'ts', 'tsx'] as const)(
  'ignores text-only package mentions when using %s',
  (loader) => {
    const source = '// @devup-ui/react\nexport const value = 1'
    const found = importsCompiledPackage(source, loader)
    expect(found).toBe(false)
  },
)

it('leaves syntax errors to Bun when the scanner rejects source', () => {
  const found = importsCompiledPackage('import {', 'ts')
  expect(found).toBe(false)
})

it('propagates unexpected scanner failures rather than treating them as syntax errors', () => {
  const failure = Symbol('scanner failure')
  const scanner = spyOn(
    Bun.Transpiler.prototype,
    'scanImports',
  ).mockImplementation(() => {
    throw failure
  })
  let received: unknown
  try {
    importsCompiledPackage('export const value = 1', 'ts')
  } catch (cause) {
    if (cause !== failure) throw cause
    received = cause
  } finally {
    scanner.mockRestore()
  }
  expect(received).toBe(failure)
})

it('distinguishes styling MDX when a compiler is missing', () => {
  const results = ['# hello', "import { Box } from '@devup-ui/react'"].map(
    mentionsCompiledPackage,
  )
  expect(results).toEqual([false, true])
})

it('preserves unique dependency edges when extraction erased the imports', () => {
  const filename = join(process.cwd(), 'src', 'entry.ts')
  const dependency = join(process.cwd(), 'tokens.ts')
  const code = preserveDependencies('export const cls = "a"', filename, [
    dependency,
    dependency,
    join(process.cwd(), 'src', 'local.ts'),
  ])
  expect(
    new Bun.Transpiler({ loader: 'ts' })
      .scanImports(code)
      .map(({ path }) => path),
  ).toEqual(['../tokens.ts', './local.ts'])
})

it('retains source when no dependencies were extracted', () => {
  const code = preserveDependencies('export const value = 1', 'entry.ts', [])
  expect(code).toBe('export const value = 1')
})

it('claims only candidate files when building a runtime filter', () => {
  const filter = runtimeSourceFilter(['C:\\source\\foo.bar.ts'])
  expect(filter.test('C:\\source\\foo.bar.ts')).toBe(true)
  expect(filter.test('C:/source/foo.bar.ts')).toBe(true)
  expect(filter.test('C:/source/fooXbar.ts')).toBe(false)
  expect(runtimeSourceFilter([]).test('anything.ts')).toBe(false)
})
