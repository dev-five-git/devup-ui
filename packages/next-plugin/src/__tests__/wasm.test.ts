import {
  mkdirSync,
  mkdtempSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import * as wasm from '@devup-ui/wasm'
import * as webpackPlugin from '@devup-ui/webpack-plugin'
import { afterAll, afterEach, beforeAll, describe, expect, it } from 'bun:test'

import {
  loadWasm,
  loadWebpackPlugin,
  requireFromPlugin,
  requireWasm,
  setWasmForTesting,
  setWebpackPluginForTesting,
} from '../wasm'

const originalCwd = process.cwd()
let tempRoots: string[] = []

beforeAll(() => {
  tempRoots = []
})

afterEach(() => {
  process.chdir(originalCwd)
  setWasmForTesting(undefined)
  setWebpackPluginForTesting(undefined)
})

afterAll(() => {
  for (const root of tempRoots) rmSync(root, { recursive: true, force: true })
})

describe('WASM selection', () => {
  it('uses an injected namespace in tests', () => {
    setWasmForTesting(wasm)
    expect(loadWasm(true)).toBe(wasm)
    expect(loadWasm(false)).toBe(wasm)
  })

  it('loads the full and lite package exports', () => {
    expect(typeof loadWasm(false).codeExtract).toBe('function')
    expect(typeof loadWasm(true).codeExtract).toBe('function')
  })

  it('resolves dependencies from a Bun-style isolated install', () => {
    const root = mkdtempSync(join(tmpdir(), 'devup-ui-next-isolated-'))
    tempRoots.push(root)
    const isolatedNodeModules = join(
      root,
      'node_modules/.bun/next-plugin/node_modules',
    )
    const pluginDir = join(isolatedNodeModules, '@devup-ui/next-plugin')
    const wasmDir = join(isolatedNodeModules, '@devup-ui/wasm')
    const rootScope = join(root, 'node_modules/@devup-ui')
    mkdirSync(pluginDir, { recursive: true })
    mkdirSync(wasmDir, { recursive: true })
    mkdirSync(rootScope, { recursive: true })
    writeFileSync(
      join(pluginDir, 'package.json'),
      JSON.stringify({ name: '@devup-ui/next-plugin' }),
    )
    writeFileSync(
      join(wasmDir, 'package.json'),
      JSON.stringify({ name: '@devup-ui/wasm', main: 'index.cjs' }),
    )
    writeFileSync(
      join(wasmDir, 'index.cjs'),
      'module.exports = { isolated: true }',
    )
    symlinkSync(
      pluginDir,
      join(rootScope, 'next-plugin'),
      process.platform === 'win32' ? 'junction' : 'dir',
    )

    process.chdir(root)

    expect(requireFromPlugin<{ isolated: boolean }>('@devup-ui/wasm')).toEqual({
      isolated: true,
    })
  })

  it.each(['MODULE_NOT_FOUND', 'ERR_PACKAGE_PATH_NOT_EXPORTED'])(
    'falls back to the full package when the lite export fails with %s',
    (code) => {
      const specifiers: string[] = []
      const loaded = requireWasm(true, (specifier) => {
        specifiers.push(specifier)
        if (specifier.endsWith('/lite')) {
          const message =
            code === 'ERR_PACKAGE_PATH_NOT_EXPORTED'
              ? "Package subpath './lite' is not defined by exports in node_modules/@devup-ui/wasm/package.json"
              : "Cannot find module '@devup-ui/wasm/lite'"
          throw Object.assign(new Error(message), { code })
        }
        return wasm
      })

      expect(loaded).toBe(wasm)
      expect(specifiers).toEqual(['@devup-ui/wasm/lite', '@devup-ui/wasm'])
    },
  )

  it('does not hide failures while loading an available lite export', () => {
    const loadError = Object.assign(new Error('invalid WASM binary'), {
      code: 'MODULE_NOT_FOUND',
    })

    expect(() =>
      requireWasm(true, () => {
        throw loadError
      }),
    ).toThrow(loadError)
  })

  it('loads or injects the Webpack plugin without a static dependency', () => {
    expect(typeof loadWebpackPlugin().DevupUIWebpackPlugin).toBe('function')
    setWebpackPluginForTesting(webpackPlugin)
    expect(loadWebpackPlugin()).toBe(webpackPlugin)
  })
})
