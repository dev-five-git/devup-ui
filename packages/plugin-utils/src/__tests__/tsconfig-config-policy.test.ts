import { createRequire } from 'node:module'
import { join, resolve } from 'node:path'

import { expect, it, spyOn } from 'bun:test'

import { resolvePackage } from '../owned-module-resolution'
import { readPackageManifest } from '../packaged-config-inputs'
import { createPreparedFixture } from './prepared-graph-fixture'

const ts: {
  readonly sys: object
  nodeNextJsonConfigResolver(
    request: string,
    importer: string,
    host: object,
  ): { readonly resolvedModule?: { readonly resolvedFileName: string } }
} = createRequire(import.meta.url)('@typescript/typescript6')
let root: string
const file = createPreparedFixture((directory) => {
  root = directory
})

it('uses a nested package config before root versions when root has no exports property', () => {
  const importer = file('tsconfig.json', '{}')
  file(
    'node_modules/preset/package.json',
    '{"typesVersions":{"*":{"config":["red.json"]}}}',
  )
  file('node_modules/preset/red.json', '{}')
  file('node_modules/preset/config/package.json', '{"tsconfig":"blue.json"}')
  const expected = file('node_modules/preset/config/blue.json', '{}')
  const oracle = ts.nodeNextJsonConfigResolver(
    'preset/config',
    importer,
    ts.sys,
  ).resolvedModule?.resolvedFileName
  expect(oracle && resolve(oracle)).toBe(expected)
  expect(
    resolvePackage('preset/config', importer, { purpose: 'tsconfig-extends' }),
  ).toBe(expected)
})

it('returns a missing imports result when no package scope exists', () => {
  const importer = file('tsconfig.json', '{}')
  const oracle = ts.nodeNextJsonConfigResolver('#missing', importer, ts.sys)
    .resolvedModule?.resolvedFileName
  expect(oracle).toBeUndefined()
  expect(
    resolvePackage('#missing', importer, { purpose: 'tsconfig-extends' }),
  ).toBeUndefined()
})

it('preserves non-syntax parser failures rather than translating them to empty manifests', () => {
  const manifest = file('package.json', '{}')
  const failure = new TypeError('parser failure')
  const parse = spyOn(JSON, 'parse').mockImplementation(() => {
    throw failure
  })
  try {
    expect(() => readPackageManifest(manifest, 'tsconfig-extends')).toThrow(
      failure,
    )
  } finally {
    parse.mockRestore()
  }
})

it('ignores package exports when the compiler takes the dot filesystem-directory branch', () => {
  // Given a directory config and a distinct exported JSON in the same directory.
  const importer = file('src/tsconfig.json', '{"extends":"."}')
  file('src/package.json', '{"exports":"./blue.json"}')
  file('src/blue.json', '{}')
  // When the packaged entry delegates dot requests to its CJS directory policy.
  const oracle = ts.nodeNextJsonConfigResolver('.', importer, ts.sys)
    .resolvedModule?.resolvedFileName
  expect(oracle && resolve(oracle)).toBe(importer)
  // Then exports encapsulation does not govern the filesystem-directory branch.
  expect(resolvePackage('.', importer, { purpose: 'tsconfig-extends' })).toBe(
    importer,
  )
})

it('preserves compiler casing identity when realpath differs only in case', () => {
  // Given an exports target whose casing differs from the real directory entry.
  const importer = file('tsconfig.json', '{}')
  file('node_modules/preset/package.json', '{"exports":"./BLUE.json"}')
  file('node_modules/preset/blue.json', '{}')
  const expected =
    process.platform === 'win32'
      ? join(root, 'node_modules/preset/BLUE.json')
      : undefined
  // When both resolvers use the actual case-sensitive/case-insensitive filesystem.
  const oracle = ts.nodeNextJsonConfigResolver('preset', importer, ts.sys)
    .resolvedModule?.resolvedFileName
  expect(oracle && resolve(oracle)).toBe(expected)
  // Then canonicalization changes only genuine symlink origins, not spelling-only identity.
  expect(
    resolvePackage('preset', importer, { purpose: 'tsconfig-extends' }),
  ).toBe(expected)
})

it('applies root versions to dot-prefixed child directories without treating them as parent escapes', () => {
  // Given an in-package field path whose child name starts with two dots.
  const importer = file('tsconfig.json', '{}')
  file(
    'node_modules/preset/package.json',
    '{"tsconfig":"..configs/red.json","typesVersions":{"*":{"..configs/red.json":["blue.json"]}}}',
  )
  file('node_modules/preset/..configs/red.json', '{}')
  const expected = file('node_modules/preset/blue.json', '{}')
  // When resolving versions before the package field.
  const oracle = ts.nodeNextJsonConfigResolver('preset', importer, ts.sys)
    .resolvedModule?.resolvedFileName
  expect(oracle && resolve(oracle)).toBe(expected)
  // Then only an actual parent path component excludes version remapping.
  expect(
    resolvePackage('preset', importer, { purpose: 'tsconfig-extends' }),
  ).toBe(expected)
})
