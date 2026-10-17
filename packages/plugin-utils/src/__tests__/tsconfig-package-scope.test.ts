import * as fs from 'node:fs'
import { createRequire } from 'node:module'
import { dirname, join, resolve } from 'node:path'

import { expect, it, spyOn } from 'bun:test'

import { createModuleResolver } from '../import-graph'
import { resolvePackage } from '../owned-module-resolution'
import { createResolutionInputs } from '../resolution-inputs'
import { readPathAliases } from '../tsconfig'
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

it.each([
  ['self', 'app', { name: 'app', exports: './blue.json' }, 'blue.json'],
  [
    'self subpath',
    'app/base',
    { name: 'app', exports: { './base': './blue.json' } },
    'blue.json',
  ],
  ['imports', '#base', { imports: { '#base': './blue.json' } }, 'blue.json'],
  [
    'imports pattern',
    '#config/blue',
    { imports: { '#config/*': './*.json' } },
    'blue.json',
  ],
  [
    'imports bare target ignores main and tsconfig',
    '#base',
    { imports: { '#base': 'preset' } },
    undefined,
  ],
  ['self unavailable', 'app', { name: 'app', exports: {} }, undefined],
  ['imports unavailable', '#missing', { imports: {} }, undefined],
  ['imports null', '#base', { imports: { '#base': null } }, undefined],
  [
    'imports parent prohibited',
    '#base',
    { imports: { '#base': '../blue.json' } },
    undefined,
  ],
  [
    'imports absolute prohibited',
    '#base',
    { imports: { '#base': '/blue.json' } },
    undefined,
  ],
  ['invalid hash', '#', {}, undefined],
  ['URI misses', 'https:blue', {}, undefined],
  ['dot CJS branch', '.', {}, 'src/tsconfig.json'],
  ['parent CJS branch', '..', {}, 'tsconfig.json'],
] as const)(
  'matches compiler nearest scope when %s',
  (_, request, manifest, expected) => {
    // Given a nearest scope and distinct real entry files.
    const importer = file(
      'src/tsconfig.json',
      JSON.stringify({ extends: request }),
    )
    file('package.json', JSON.stringify(manifest))
    file('tsconfig.json', '{}')
    file('blue.json', '{}')
    file(
      'node_modules/preset/package.json',
      '{"main":"blue.json","tsconfig":"red.json"}',
    )
    file('node_modules/preset/blue.json', '{}')
    file('node_modules/preset/red.json', '{}')
    // When resolving with the installed compiler and the shared owned engine.
    const target = expected === undefined ? undefined : join(root, expected)
    const oracle = ts.nodeNextJsonConfigResolver(request, importer, ts.sys)
      .resolvedModule?.resolvedFileName
    expect(oracle && resolve(oracle)).toBe(target)
    // Then both select the independent expected target, including misses.
    expect(
      resolvePackage(request, importer, { purpose: 'tsconfig-extends' }),
    ).toBe(target)
  },
)

it('keeps selected physical origins while recording lexical scope and target probes', () => {
  // Given a scoped junction with nested inheritance and relative alias origins.
  const config = file('src/tsconfig.json', '{"extends":"@scope/preset"}')
  const manifest = file(
    'workspace/preset/package.json',
    '{"exports":"./nested/base.json"}',
  )
  const base = file(
    'workspace/preset/nested/base.json',
    '{"extends":"../shared.json"}',
  )
  const shared = file(
    'workspace/preset/shared.json',
    '{"compilerOptions":{"paths":{"color":["value.ts"]}}}',
  )
  const value = file('workspace/preset/value.ts', 'blue')
  file('node_modules/@scope/placeholder.json', '{}')
  fs.symlinkSync(
    dirname(manifest),
    join(root, 'node_modules/@scope/preset'),
    'junction',
  )
  const inputs = createResolutionInputs()
  // When inheritance selects through the owned resolver.
  const aliases = readPathAliases(config, inputs)
  // Then physical config owns relative paths but lexical selecting IO remains repairable.
  expect(aliases.aliases[0]?.targets).toEqual([value])
  expect(inputs.snapshot().fileDependencies).toEqual(
    [
      config,
      base,
      shared,
      join(root, 'node_modules/@scope/preset/package.json'),
      join(root, 'node_modules/@scope/preset/nested/base.json'),
    ].sort(),
  )
})

it('reuses frozen setup evidence without rereading inheritance during repeated imports', () => {
  // Given a public resolver and a narrow real read spy.
  file('tsconfig.json', '{"extends":"preset"}')
  file('node_modules/preset/package.json', '{"exports":"./base.json"}')
  file(
    'node_modules/preset/base.json',
    '{"compilerOptions":{"paths":{"color":["value.ts"]}}}',
  )
  const value = file('node_modules/preset/value.ts', 'blue')
  const reads: string[] = []
  const originalRead = fs.readFileSync
  const read = spyOn(fs, 'readFileSync').mockImplementation(
    new Proxy(originalRead, {
      apply(target, receiver, args) {
        reads.push(String(args[0]))
        return Reflect.apply(target, receiver, args)
      },
    }),
  )
  try {
    const resolver = createModuleResolver({ cwd: root })
    reads.length = 0
    // When repeated imports use the same setup.
    for (let i = 0; i < 3; i++)
      expect(resolver('color', 'main.ts')?.path).toBe(value)
    // Then only source content, never configs or selecting manifests, is read.
    expect(reads).toEqual([value, value, value])
  } finally {
    read.mockRestore()
  }
})

it('locates a cyclic imports map instead of exhausting the native stack', () => {
  const importer = file('tsconfig.json', '{"extends":"#base"}')
  file('package.json', '{"imports":{"#base":"#base"}}')
  expect(() =>
    ts.nodeNextJsonConfigResolver('#base', importer, ts.sys),
  ).toThrow(RangeError)
  expect(() => readPathAliases(importer)).toThrow(
    `${join(root, 'package.json')}:1:1: Cannot load configuration: Package imports cycle`,
  )
})

it.each(['exports', 'manifestless', 'nested'])(
  'searches outward after an unresolvable nearer %s package',
  (mode) => {
    // Given a nearer existing package and an independently resolving outer candidate.
    const importer = file('src/tsconfig.json', '{}')
    const request = mode === 'nested' ? 'preset/config' : 'preset'
    file(
      'src/node_modules/preset/package.json',
      mode === 'exports' ? '{"exports":{}}' : '{}',
    )
    file('node_modules/preset/package.json', '{}')
    const expected = file(
      `node_modules/preset/${mode === 'nested' ? 'config/' : ''}tsconfig.json`,
      '{}',
    )
    // When trying the complete ancestor search.
    const oracle = ts.nodeNextJsonConfigResolver(request, importer, ts.sys)
      .resolvedModule?.resolvedFileName
    // Then nearest-manifest existence cannot prematurely terminate resolution.
    expect(oracle && resolve(oracle)).toBe(expected)
    expect(
      resolvePackage(request, importer, { purpose: 'tsconfig-extends' }),
    ).toBe(expected)
  },
)

it.each(['.preset', '@scope/preset'])(
  'treats %s as a package rather than a relative path',
  (request) => {
    // Given a legal package name with its default config.
    const importer = file('tsconfig.json', JSON.stringify({ extends: request }))
    file(`node_modules/${request}/package.json`, '{}')
    const target = file(`node_modules/${request}/tsconfig.json`, '{}')
    // When resolving inheritance.
    const oracle = ts.nodeNextJsonConfigResolver(request, importer, ts.sys)
      .resolvedModule?.resolvedFileName
    expect(oracle && resolve(oracle)).toBe(target)
    // Then Devup uses the same package selection.
    expect(readPathAliases(importer).aliases).toEqual([])
  },
)
