import { createRequire } from 'node:module'

import { expect, it } from 'bun:test'

import { createModuleResolver } from '../import-graph'
import { parseJsonc } from '../jsonc'
import { ConfigLoadError } from '../load-config'
import { readPackageManifest } from '../packaged-config-inputs'
import { createPreparedFixture } from './prepared-graph-fixture'

const ts: {
  readonly version: string
  parseConfigFileTextToJson(
    file: string,
    source: string,
  ): {
    readonly config?: unknown
    readonly error?: unknown
  }
} = createRequire(import.meta.url)('@typescript/typescript6')
const hexOverflow = `0x${'F'.repeat(400)}`
let root: string
const file = createPreparedFixture((directory) => {
  root = directory
})

it.each([
  ['0xff', 255],
  ['-0xff', -255],
  ['1_000', 1000],
  ['-1_000', -1000],
  ['0b10', 2],
  ['0o10', 8],
  [hexOverflow, Infinity],
  [`-${hexOverflow}`, -Infinity],
  ['1_0e999', Infinity],
  ['-1_0e999', -Infinity],
  ['1e999', Infinity],
  ['-1e999', -Infinity],
] as const)(
  'retains numeric value and unrelated selection metadata for %s',
  (literal, expected) => {
    // Given valid JSONC numeric syntax and independently expected selecting/numeric fields.
    expect(ts.version).toBe('6.0.3')
    const source = `{"tsconfig":"blue.json","other":${literal}}`
    const fields = { tsconfig: 'blue.json', other: expected }
    // When the installed compiler and owned parser convert the same literal.
    const oracle = ts.parseConfigFileTextToJson('package.json', source)
    // Then neither drops metadata nor caps/nulls the exact numeric result.
    expect(oracle.error).toBeUndefined()
    expect(oracle.config).toEqual(fields)
    expect(parseJsonc(source, true)).toEqual(fields)
  },
)

it.each([hexOverflow, '1_0e999', '0xff', '1_000'])(
  'keeps ordinary module manifests strict for JSONC literal %s',
  (literal) => {
    // Given a real module manifest whose numeric spelling requires the config-only JSONC policy.
    const manifest = file(
      'node_modules/preset/package.json',
      `{"main":"blue.js","other":${literal}}`,
    )
    file('node_modules/preset/blue.js', "export const color='blue'")
    // When the ordinary module purpose encounters that non-JSON spelling.
    expect(() => readPackageManifest(manifest, 'module')).toThrow(
      ConfigLoadError,
    )
    // Then the unchanged public module resolver retains the real manifest load failure.
    expect(() =>
      createModuleResolver({ cwd: root })('preset', 'main.ts'),
    ).toThrow(manifest)
  },
)

it('preserves standard JSON decimal overflow in ordinary module manifests', () => {
  // Given valid standard JSON with an unrelated overflow field and a real main module.
  const manifest = file(
    'node_modules/preset/package.json',
    '{"main":"blue.js","other":1e999}',
  )
  const target = file(
    'node_modules/preset/blue.js',
    "export const color='blue'",
  )
  // When the unchanged strict JSON policy and module resolver read it.
  const fields = readPackageManifest(manifest, 'module')
  // Then standard JSON semantics retain Infinity and ordinary main selection.
  expect(fields).toEqual({ main: 'blue.js', other: Infinity })
  expect(createModuleResolver({ cwd: root })('preset', 'main.ts')?.path).toBe(
    target,
  )
})

it.each(['0x', '1__0'])(
  'keeps malformed numeric literal %s as a config-manifest parse miss',
  (literal) => {
    // Given malformed syntax beside a selecting field.
    const source = `{"tsconfig":"blue.json","other":${literal}}`
    const manifest = file('package.json', source)
    // When the config-only parser and manifest boundary encounter the invalid literal.
    expect(() => parseJsonc(source, true)).toThrow(SyntaxError)
    // Then the established malformed-manifest fallback remains an empty record.
    expect(readPackageManifest(manifest, 'tsconfig-extends')).toEqual({})
  },
)
