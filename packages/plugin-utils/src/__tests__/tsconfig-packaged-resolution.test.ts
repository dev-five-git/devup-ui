import { createRequire } from 'node:module'
import { join, resolve } from 'node:path'

import { expect, it } from 'bun:test'

import { readPathAliases } from '../tsconfig'
import { createPreparedFixture } from './prepared-graph-fixture'
import { manifestCases } from './tsconfig-manifest-cases'

const ts: {
  readonly version: string
  readonly sys: object
  nodeNextJsonConfigResolver(
    request: string,
    importer: string,
    host: object,
  ): {
    readonly resolvedModule?: { readonly resolvedFileName: string }
  }
} = createRequire(import.meta.url)('@typescript/typescript6')
let root: string
const file = createPreparedFixture((directory) => {
  root = directory
})

const cases: readonly [string, unknown, string | undefined, string?][] = [
  ['ignores JSON main', { main: 'red.json' }, 'tsconfig.json'],
  [
    'ignores module/types fields',
    { module: 'red.json', types: 'red.json' },
    'tsconfig.json',
  ],
  ['uses tsconfig field', { tsconfig: 'blue.json' }, 'blue.json'],
  ['appends field extension', { tsconfig: 'blue' }, 'blue.json'],
  [
    'falls back after field miss',
    { tsconfig: 'missing.json' },
    'tsconfig.json',
  ],
  ['ignores empty field', { tsconfig: '' }, 'tsconfig.json'],
  ['ignores non-string field', { tsconfig: 1 }, 'tsconfig.json'],
  [
    'exports precedes field',
    { exports: './blue.json', tsconfig: 'red.json' },
    'blue.json',
  ],
  [
    'types precedes require by insertion',
    {
      exports: {
        import: './red.json',
        types: './blue.json',
        require: './red.json',
      },
    },
    'blue.json',
  ],
  [
    'require precedes types by insertion',
    { exports: { require: './red.json', types: './blue.json' } },
    'red.json',
  ],
  [
    'node is active',
    { exports: { module: './red.json', node: './blue.json' } },
    'blue.json',
  ],
  [
    'early default wins',
    { exports: { default: './red.json', types: './blue.json' } },
    'red.json',
  ],
  [
    'condition miss advances',
    { exports: { require: './missing.json', types: './blue.json' } },
    'blue.json',
  ],
  [
    'condition null blocks',
    { exports: { require: null, types: './blue.json' } },
    undefined,
  ],
  [
    'array miss advances',
    { exports: ['./missing.json', './blue.json'] },
    'blue.json',
  ],
  [
    'invalid array target advances',
    { exports: ['../red.json', './blue.json'] },
    'blue.json',
  ],
  ['array null blocks', { exports: [null, './blue.json'] }, undefined],
  ['empty array misses', { exports: [] }, undefined],
  ['false target advances', { exports: [false, './blue.json'] }, 'blue.json'],
  ['null exports falls back', { exports: null }, 'tsconfig.json'],
  ['false exports falls back', { exports: false }, 'tsconfig.json'],
  ['empty exports falls back', { exports: '' }, 'tsconfig.json'],
  ['empty map encapsulates', { exports: {} }, undefined],
  ['root map encapsulates', { exports: { './red': './red.json' } }, undefined],
  ['export JS substitutes JSON', { exports: './red.js' }, 'red.json'],
  ['export TS substitutes JSON', { exports: './red.d.ts' }, 'red.json'],
  ['export extensionless stays exact', { exports: './blue' }, undefined],
  ['export directory stays exact', { exports: './dir' }, undefined],
  ['explicit subpath resolves', {}, 'blue.json', 'preset/blue.json'],
  ['deep JS substitutes JSON', {}, 'red.json', 'preset/red.js'],
  ['deep unknown appends JSON', {}, 'red.xyz.json', 'preset/red.xyz'],
  ['deep MJS appends JSON', {}, 'red.mjs.json', 'preset/red.mjs'],
  ['directory default is tsconfig', {}, 'dir/tsconfig.json', 'preset/dir'],
  ['field directory default', { tsconfig: 'dir' }, 'dir/tsconfig.json'],
  [
    'exact precedes patterns',
    { exports: { './*': './red.json', './blue': './blue.json' } },
    'blue.json',
    'preset/blue',
  ],
  [
    'longest pattern wins',
    { exports: { './*': './red.json', './config/*': './blue.json' } },
    'blue.json',
    'preset/config/value',
  ],
  [
    'pattern trailer matches',
    { exports: { './*.json': './*.json' } },
    'blue.json',
    'preset/blue.json',
  ],
  [
    'slash prefix matches',
    { exports: { './config/': './' } },
    'blue.json',
    'preset/config/blue.json',
  ],
  [
    'slash target needs slash',
    { exports: { './config/': './blue.json' } },
    undefined,
    'preset/config/blue.json',
  ],
  [
    'mixed map root follows dot branch',
    { exports: { types: './red.json', '.': './blue.json' } },
    'blue.json',
  ],
  [
    'mixed map deep is unavailable',
    { exports: { types: './red.json', './blue': './blue.json' } },
    undefined,
    'preset/blue',
  ],
  ['invalid target dot segment', { exports: './dir/../blue.json' }, undefined],
  [
    'invalid target node_modules segment',
    { exports: './dir/node_modules/blue.json' },
    undefined,
  ],
  ...manifestCases,
]

it.each(cases)(
  'matches installed TypeScript when %s',
  (_, manifest, expected, request = 'preset') => {
    // Given independent, distinct config targets and the installed compiler oracle.
    expect(ts.version).toBe('6.0.3')
    const config = file('tsconfig.json', JSON.stringify({ extends: request }))
    file(
      'node_modules/preset/package.json',
      typeof manifest === 'string' ? manifest : JSON.stringify(manifest),
    )
    for (const target of [
      'red.json',
      'blue.json',
      'tsconfig.json',
      'dir/tsconfig.json',
      'red.xyz.json',
      'red.mjs.json',
    ])
      file(
        `node_modules/preset/${target}`,
        JSON.stringify({ compilerOptions: { paths: { selected: [target] } } }),
      )
    const target =
      expected === undefined
        ? undefined
        : join(root, 'node_modules/preset', expected)
    // When both resolvers select against the same real files without a shared cache.
    const oracle = ts.nodeNextJsonConfigResolver(request, config, ts.sys)
      .resolvedModule?.resolvedFileName
    // Then both must match the fixture's independent expected target (not each other alone).
    expect(oracle === undefined ? undefined : resolve(oracle)).toBe(target)
    if (target === undefined)
      expect(() => readPathAliases(config)).toThrow('Cannot load configuration')
    else
      expect(readPathAliases(config).aliases[0]?.targets[0]).toBe(
        resolve(join(target, '..'), expected ?? ''),
      )
  },
)
