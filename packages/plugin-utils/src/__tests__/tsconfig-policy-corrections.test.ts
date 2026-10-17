import { createRequire } from 'node:module'
import { join, resolve } from 'node:path'

import { expect, it } from 'bun:test'

import { createModuleResolver } from '../import-graph'
import { resolvePackage } from '../owned-module-resolution'
import { createPreparedFixture } from './prepared-graph-fixture'

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
const red = '{"compilerOptions":{"paths":{"color":["red.ts"]}}}'
const blue = '{"compilerOptions":{"paths":{"color":["blue.ts"]}}}'

it.each([false, true])(
  'retains root versions in manifestless child directories (root field=%s)',
  (withField) => {
    // Given child configs with distinct aliases and root-only version metadata.
    const importer = file('tsconfig.json', '{"extends":"preset/dir"}')
    file(
      'node_modules/preset/package.json',
      JSON.stringify({
        ...(withField ? { tsconfig: 'red.json' } : {}),
        typesVersions: { '*': { tsconfig: ['blue.json'] } },
      }),
    )
    file('node_modules/preset/red.json', red)
    file('node_modules/preset/dir/red.json', red)
    file('node_modules/preset/dir/tsconfig.json', red)
    const selected = file('node_modules/preset/dir/blue.json', blue)
    file('node_modules/preset/dir/red.ts', "export const color='red'")
    const target = file(
      'node_modules/preset/dir/blue.ts',
      "export const color='blue'",
    )
    // When resolving the child directory without a child package.json.
    const oracle = ts.nodeNextJsonConfigResolver('preset/dir', importer, ts.sys)
      .resolvedModule?.resolvedFileName
    // Then root versions apply relative to the child, but its root tsconfig field does not.
    expect(oracle && resolve(oracle)).toBe(selected)
    expect(
      resolvePackage('preset/dir', importer, { purpose: 'tsconfig-extends' }),
    ).toBe(selected)
    expect(createModuleResolver({ cwd: root })('color', 'main.ts')?.path).toBe(
      target,
    )
  },
)

it.each([
  [
    'miss',
    './missing.json',
    'node_modules/preset/blue.json',
    'node_modules/preset/blue.ts',
  ],
  ['resolved', './red.json', 'red.json', 'red.ts'],
  ['blocked', { require: null, default: './red.json' }, undefined, undefined],
] as const)(
  'distinguishes self exports %s from installed same-name fallback',
  (_, exports, selectedPath, targetPath) => {
    // Given a self scope and an independently resolving same-name installed package.
    const importer = file('tsconfig.json', '{"extends":"preset"}')
    file('package.json', JSON.stringify({ name: 'preset', exports }))
    file('red.json', red)
    file('red.ts', "export const color='red'")
    file('node_modules/preset/package.json', '{"exports":"./blue.json"}')
    file('node_modules/preset/blue.json', blue)
    file('node_modules/preset/blue.ts', "export const color='blue'")
    const selected =
      selectedPath === undefined ? undefined : join(root, selectedPath)
    // When the nearest self lookup returns a resolved target, a miss, or actual null block.
    const oracle = ts.nodeNextJsonConfigResolver('preset', importer, ts.sys)
      .resolvedModule?.resolvedFileName
    // Then only a miss reaches installed fallback; a real block stays unresolved.
    expect(oracle && resolve(oracle)).toBe(selected)
    expect(
      resolvePackage('preset', importer, { purpose: 'tsconfig-extends' }),
    ).toBe(selected)
    if (targetPath === undefined)
      expect(() => createModuleResolver({ cwd: root })).toThrow(
        'Cannot resolve tsconfig extends',
      )
    else
      expect(
        createModuleResolver({ cwd: root })('color', 'main.ts')?.path,
      ).toBe(join(root, targetPath))
  },
)

it.each([
  ['./dir\\..\\blue.json', undefined],
  ['./dir\\blue.json', 'node_modules/preset/dir/blue.json'],
  ['.\\blue.json', undefined],
] as const)(
  'normalizes export components without broadening raw eligibility for %s',
  (exports, selectedPath) => {
    // Given both escaped and ordinary candidate files, with exact raw export syntax.
    const importer = file('tsconfig.json', '{}')
    file('node_modules/preset/package.json', JSON.stringify({ exports }))
    file('node_modules/preset/blue.json', blue)
    file('node_modules/preset/dir/blue.json', blue)
    const selected =
      selectedPath === undefined ? undefined : join(root, selectedPath)
    // When TypeScript and the owned engine validate then resolve components.
    const oracle = ts.nodeNextJsonConfigResolver('preset', importer, ts.sys)
      .resolvedModule?.resolvedFileName
    // Then backslash parent components fail while a valid raw ./ prefix can resolve normalized children.
    expect(oracle && resolve(oracle)).toBe(selected)
    expect(
      resolvePackage('preset', importer, { purpose: 'tsconfig-extends' }),
    ).toBe(selected)
  },
)

it.each([
  [
    'preset/',
    'node_modules/preset/tsconfig.json',
    'node_modules/preset/blue.ts',
  ],
  ['preset', 'node_modules/preset.json', 'node_modules/red.ts'],
  [
    'preset/dir/',
    'node_modules/preset/dir/tsconfig.json',
    'node_modules/preset/dir/blue.ts',
  ],
] as const)(
  'preserves directory identity and bare sibling precedence for %s',
  (request, selectedPath, targetPath) => {
    // Given existing sibling JSON files and distinct directory configs.
    const importer = file('tsconfig.json', JSON.stringify({ extends: request }))
    file('node_modules/preset.json', red)
    file('node_modules/red.ts', "export const color='red'")
    file('node_modules/preset/package.json', '{}')
    file('node_modules/preset/tsconfig.json', blue)
    file('node_modules/preset/blue.ts', "export const color='blue'")
    file('node_modules/preset/dir.json', red)
    file('node_modules/preset/dir/tsconfig.json', blue)
    file('node_modules/preset/dir/blue.ts', "export const color='blue'")
    // When selecting a bare or trailing-separator request.
    const oracle = ts.nodeNextJsonConfigResolver(request, importer, ts.sys)
      .resolvedModule?.resolvedFileName
    // Then the request's independent file/directory priority determines selection.
    expect(oracle && resolve(oracle)).toBe(join(root, selectedPath))
    expect(
      resolvePackage(request, importer, { purpose: 'tsconfig-extends' }),
    ).toBe(join(root, selectedPath))
    expect(createModuleResolver({ cwd: root })('color', 'main.ts')?.path).toBe(
      join(root, targetPath),
    )
  },
)
