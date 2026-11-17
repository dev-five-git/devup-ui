import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'

import { afterEach, beforeEach, describe, expect, it } from 'bun:test'

import {
  esmImportConditions,
  viteImportConditions,
} from '../bundler-conditions'
import { createModuleResolver } from '../import-graph'

describe('esmImportConditions', () => {
  it('expands the ESM conditions with the general ones', () => {
    expect(
      esmImportConditions({
        conditionNames: ['webpack', 'production', 'browser'],
        byDependency: {
          esm: { conditionNames: ['import', 'module', '...'] },
          commonjs: { conditionNames: ['require', '...'] },
        },
      }),
    ).toEqual(['import', 'module', 'webpack', 'production', 'browser'])
  })

  it('falls back to the general conditions and then to none', () => {
    expect(esmImportConditions({ conditionNames: ['browser'] })).toEqual([
      'browser',
    ])
    expect(
      esmImportConditions({
        byDependency: { esm: { conditionNames: ['...'] } },
      }),
    ).toEqual([])
    expect(esmImportConditions({ byDependency: {} })).toBeUndefined()
    expect(esmImportConditions(undefined)).toBeUndefined()
  })
})

describe('viteImportConditions', () => {
  it('adds import and resolves the mode placeholder', () => {
    const conditions = ['module', 'browser', 'development|production']
    expect(
      viteImportConditions({ isProduction: true, resolve: { conditions } }),
    ).toEqual(['import', 'module', 'browser', 'production'])
    expect(viteImportConditions({ resolve: { conditions } })).toEqual([
      'import',
      'module',
      'browser',
      'development',
    ])
  })

  it('names none when the config does not', () => {
    expect(viteImportConditions({ resolve: {} })).toBeUndefined()
    expect(viteImportConditions(undefined)).toBeUndefined()
  })
})

describe('createModuleResolver conditions', () => {
  let root: string

  beforeEach(() => {
    root = mkdtempSync(join(tmpdir(), 'devup-ui-conditions-'))
    const files: Record<string, string> = {
      'node_modules/split/package.json': JSON.stringify({
        exports: {
          '.': {
            browser: './browser.js',
            import: './import.js',
            default: './default.js',
          },
          './nested': {
            node: { import: './node-import.js', default: './node.js' },
            default: './fallback.js',
          },
          './empty': { browser: {}, default: './empty-fallback.js' },
        },
      }),
      'node_modules/split/browser.js': 'browser',
      'node_modules/split/import.js': 'import',
      'node_modules/split/default.js': 'default',
      'node_modules/split/node-import.js': 'node-import',
      'node_modules/split/node.js': 'node',
      'node_modules/split/fallback.js': 'fallback',
      'node_modules/split/empty-fallback.js': 'empty-fallback',
      'node_modules/required/package.json': JSON.stringify({
        exports: { require: './index.cjs', default: null },
      }),
      'node_modules/required/index.cjs': 'required',
    }
    for (const [path, code] of Object.entries(files)) {
      mkdirSync(dirname(join(root, path)), { recursive: true })
      writeFileSync(join(root, path), code)
    }
  })

  afterEach(() => {
    rmSync(root, { recursive: true, force: true })
  })

  function code(specifier: string, conditions?: readonly string[]) {
    return createModuleResolver({ cwd: root, conditions })(
      specifier,
      'src/App.tsx',
    )?.code
  }

  it('takes the first matching branch in the order the package lists them', () => {
    expect(code('split', ['import', 'browser'])).toBe('browser')
    expect(code('split', ['import'])).toBe('import')
    expect(code('split', [])).toBe('default')
    expect(code('split/nested', ['import', 'node'])).toBe('node-import')
    expect(code('split/nested', ['node'])).toBe('node')
    expect(code('split/nested', ['browser'])).toBe('fallback')
    expect(code('split/empty', ['browser'])).toBe('empty-fallback')
  })

  it('resolves import, module and node conditions by default', () => {
    expect(code('split')).toBe('import')
    expect(code('split/nested')).toBe('node-import')
  })

  it('resolves a package exported only to require', () => {
    expect(code('required', ['import', 'browser'])).toBe('required')
  })
})
