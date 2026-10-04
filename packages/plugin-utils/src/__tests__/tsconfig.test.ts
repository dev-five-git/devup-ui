import {
  mkdirSync,
  mkdtempSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'

import { afterEach, beforeEach, describe, expect, it } from 'bun:test'

import { buildStaticImportGraph, createModuleResolver } from '../import-graph'
import { readPathAliases } from '../tsconfig'

const parent = join(
  tmpdir(),
  'opencode',
  'workers',
  'w20-plugins-core',
  'shared',
)
let root: string
beforeEach(() => {
  mkdirSync(parent, { recursive: true })
  root = realpathSync(mkdtempSync(join(parent, 'tsconfig-')))
})
afterEach(() => rmSync(root, { recursive: true, force: true }))

function fixture(path: string, value: unknown): string {
  const file = join(root, path)
  mkdirSync(dirname(file), { recursive: true })
  writeFileSync(file, typeof value === 'string' ? value : JSON.stringify(value))
  return file
}

describe('tsconfig inheritance', () => {
  it('resolves inherited baseUrl and paths relative to their defining config', () => {
    fixture('config/base.json', {
      compilerOptions: { baseUrl: '../', paths: { '@base/*': ['src/*'] } },
    })
    fixture('tsconfig.json', { extends: './config/base.json' })
    const value = fixture('src/value.mts', 'export const value = 1')
    const entry = fixture('src/main.ts', "import '@base/value'")
    const resolver = createModuleResolver({ cwd: root })
    expect(resolver('@base/value', entry)?.path).toBe(value)
    const graph = buildStaticImportGraph(
      join(root, 'src'),
      join(root, 'tsconfig.json'),
      { cwd: root },
    )
    expect(graph.staticImports.get(entry)).toEqual(new Set([value]))
    expect(resolver('src/value', entry)?.path).toBe(value)
  })
  it('uses paths origin without baseUrl and replaces rather than deep-merging paths', () => {
    fixture('config/base.json', {
      compilerOptions: { paths: { '@base/*': ['values/*'] } },
    })
    const value = fixture('config/values/value.cts', 'export const value = 1')
    fixture('tsconfig.json', { extends: './config/base' })
    expect(
      createModuleResolver({ cwd: root })('@base/value', 'main.ts')?.path,
    ).toBe(value)
    fixture('tsconfig.json', {
      extends: './config/base',
      compilerOptions: { paths: { '@child': ['child'] } },
    })
    const child = fixture('child.ts', 'child')
    const resolver = createModuleResolver({ cwd: root })
    expect(resolver('@base/value', 'main.ts')).toBeUndefined()
    expect(resolver('@child', 'main.ts')?.path).toBe(child)
    expect(resolver('@child-extra', 'main.ts')).toBeUndefined()
  })
  it('applies an overridden baseUrl to inherited paths and merges array parents left to right', () => {
    fixture('first.json', {
      compilerOptions: { baseUrl: './old', paths: { '@x': ['value'] } },
    })
    fixture('second.json', { compilerOptions: { paths: { '@y': ['value'] } } })
    fixture('tsconfig.json', {
      extends: ['./first.json', './second.json'],
      compilerOptions: { baseUrl: './new' },
    })
    const value = fixture('new/value.ts', 'new')
    const resolver = createModuleResolver({ cwd: root })
    expect(resolver('@y', 'main.ts')?.path).toBe(value)
    expect(resolver('@x', 'main.ts')).toBeUndefined()
  })
  it('honors exact aliases and longest wildcard prefixes without fallback to shorter aliases', () => {
    fixture('tsconfig.json', {
      compilerOptions: {
        paths: {
          '@/*': ['fallback/*'],
          '@/specific/*': ['missing/*'],
          '@/exact': ['value'],
          '@end/*/tail': ['value'],
        },
      },
    })
    fixture('fallback/specific/item.ts', 'fallback')
    const value = fixture('value.ts', 'value')
    const resolver = createModuleResolver({ cwd: root })
    expect(resolver('@/specific/item', 'main.ts')).toBeUndefined()
    expect(resolver('@/exact', 'main.ts')?.path).toBe(value)
    expect(resolver('@/exact-more', 'main.ts')).toBeUndefined()
    expect(resolver('@end/tail', 'main.ts')).toBeUndefined()
    expect(resolver('@end/a/tail', 'main.ts')?.path).toBe(value)
  })
  it('parses comments and trailing commas without damaging strings', () => {
    fixture(
      'tsconfig.json',
      '{/* comment */"compilerOptions": {"paths": {"@x": ["src/value",], // trailing\n},}, "text": "quote\\"value,}",}',
    )
    const value = fixture('src/value.ts', 'value')
    expect(createModuleResolver({ cwd: root })('@x', 'main.ts')?.path).toBe(
      value,
    )
  })
  it('inherits package configs, explicit package subpaths and package tsconfig fields', () => {
    fixture('node_modules/config/package.json', {})
    fixture('node_modules/config/tsconfig.json', {
      compilerOptions: { paths: { '@pkg': ['value'] } },
    })
    const value = fixture('node_modules/config/value.ts', 'value')
    fixture('tsconfig.json', { extends: 'config' })
    expect(createModuleResolver({ cwd: root })('@pkg', 'main.ts')?.path).toBe(
      value,
    )
    fixture('tsconfig.json', { extends: 'config/tsconfig.json' })
    expect(createModuleResolver({ cwd: root })('@pkg', 'main.ts')?.path).toBe(
      value,
    )
    fixture('node_modules/custom/package.json', {
      tsconfig: 'configs/base.json',
    })
    fixture('node_modules/custom/configs/base.json', {
      compilerOptions: { paths: { '@custom': ['value'] } },
    })
    const custom = fixture('node_modules/custom/configs/value.ts', 'custom')
    fixture('tsconfig.json', { extends: 'custom' })
    expect(
      createModuleResolver({ cwd: root })('@custom', 'main.ts')?.path,
    ).toBe(custom)
  })
  it('uses package tsconfig instead of JavaScript main when both are present', () => {
    fixture('node_modules/config/package.json', {
      main: 'index.js',
      tsconfig: 'base.json',
    })
    fixture('node_modules/config/index.js', 'module.exports = {}')
    fixture('node_modules/config/base.json', {
      compilerOptions: { paths: { '@pkg': ['value'] } },
    })
    const value = fixture('node_modules/config/value.ts', 'value')
    fixture('tsconfig.json', { extends: 'config' })
    expect(createModuleResolver({ cwd: root })('@pkg', 'main.ts')?.path).toBe(
      value,
    )
  })
  it('defaults a package with JavaScript main to tsconfig.json', () => {
    fixture('node_modules/config/package.json', { main: 'index.js' })
    fixture('node_modules/config/index.js', 'module.exports = {}')
    fixture('node_modules/config/tsconfig.json', {})
    fixture('tsconfig.json', { extends: 'config' })
    expect(readPathAliases(join(root, 'tsconfig.json')).aliases).toEqual([])
  })
  it('allows diamond inheritance without mistaking repeated parents for a cycle', () => {
    fixture('base.json', { compilerOptions: { paths: { '@pkg': ['value'] } } })
    fixture('left.json', { extends: './base' })
    fixture('right.json', { extends: './base' })
    fixture('tsconfig.json', { extends: ['./left', './right'] })
    const value = fixture('value.ts', 'value')
    expect(createModuleResolver({ cwd: root })('@pkg', 'main.ts')?.path).toBe(
      value,
    )
  })
})

describe('tsconfig load errors', () => {
  it('reports the missing default config after a package has an unresolved main entry', () => {
    fixture('node_modules/incomplete/package.json', { main: 'missing.js' })
    fixture('tsconfig.json', { extends: 'incomplete' })
    expect(() => readPathAliases(join(root, 'tsconfig.json'))).toThrow(
      'tsconfig.json',
    )
  })

  it.each([
    '{',
    'null',
    '{"extends":1}',
    '{"extends":[1]}',
    '{"compilerOptions":1}',
    '{"compilerOptions":{"baseUrl":1}}',
    '{"compilerOptions":{"paths":[]}}',
    '{"compilerOptions":{"paths":{"@x":[1]}}}',
    '{/* unterminated',
  ])('rejects invalid config %s with its location', (content) => {
    const file = fixture('tsconfig.json', content)
    expect(() => createModuleResolver({ cwd: root })).toThrow(file)
  })
  it.each(['./missing', 'not-installed'])(
    'reports missing inherited config %s',
    (parent) => {
      fixture('tsconfig.json', { extends: parent })
      expect(() => createModuleResolver({ cwd: root })).toThrow(
        'Cannot load configuration',
      )
    },
  )
  it('reports canonical config cycles instead of recursing indefinitely', () => {
    fixture('tsconfig.json', { extends: './base' })
    fixture('base.json', { extends: './tsconfig' })
    expect(() => createModuleResolver({ cwd: root })).toThrow(
      'inheritance cycle',
    )
  })
  it('reports an inherited package with no tsconfig rather than using an empty config', () => {
    fixture('node_modules/private/package.json', {
      exports: { './allowed': './base.json' },
    })
    fixture('node_modules/private/base.json', {})
    fixture('tsconfig.json', { extends: 'private' })
    expect(() => createModuleResolver({ cwd: root })).toThrow(
      'Cannot load configuration',
    )
  })
})
