import {
  mkdirSync,
  mkdtempSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { join, relative } from 'node:path'

import * as wasm from '@devup-ui/wasm'
import * as webpackPlugin from '@devup-ui/webpack-plugin'
import {
  afterAll,
  afterEach,
  beforeAll,
  describe,
  expect,
  it,
  mock,
} from 'bun:test'

import {
  loadWasm,
  loadWebpackPlugin,
  requireFromPlugin,
  resetWasmForTesting,
  setWasmForTesting,
  setWebpackPluginForTesting,
  withModuleResolver,
} from '../wasm'

let tempRoots: string[] = []

beforeAll(() => {
  tempRoots = []
})

afterEach(() => {
  resetWasmForTesting()
})

afterAll(() => {
  for (const root of tempRoots) rmSync(root, { recursive: true, force: true })
})

describe('WASM loading', () => {
  it('resolves imports to cwd-relative ids on engines that load modules', () => {
    const setModuleResolver = mock()
    const engine = { setModuleResolver } as unknown as typeof wasm
    expect(withModuleResolver(engine)).toBe(engine)
    const resolveModule = setModuleResolver.mock.calls[0]![0] as (
      specifier: string,
      importer: string,
    ) => { path: string } | undefined
    expect(resolveModule('./wasm.test', import.meta.path)?.path).toBe(
      relative(process.cwd(), import.meta.path).replaceAll('\\', '/'),
    )
    const older = {} as typeof wasm
    expect(withModuleResolver(older)).toBe(older)
  })

  it('uses an injected namespace in tests', () => {
    setWasmForTesting(wasm)
    expect(loadWasm()).toBe(wasm)
  })

  it('loads the package once', () => {
    const loaded = loadWasm()
    expect(typeof loaded.codeExtract).toBe('function')
    expect(loadWasm()).toBe(loaded)
  })

  it('installs a fresh resolver after clearing the loaded namespace cache', () => {
    const root = mkdtempSync(join(tmpdir(), 'devup-ui-next-resolver-'))
    tempRoots.push(root)
    writeFileSync(join(root, 'tokens.ts'), "export const tone = 'green'")
    const engine = loadWasm()
    engine.setModuleResolver(() => ({
      path: join(root, 'tokens.ts'),
      code: "export const tone = 'blue'",
    }))
    resetWasmForTesting()
    const fresh = loadWasm()
    fresh.codeExtract(
      join(root, 'view.tsx'),
      "import { Box } from '@devup-ui/react'; import { tone } from './tokens'; export const view = <Box bg={tone} />",
      '@devup-ui/react',
      'df',
      true,
      false,
      false,
      {},
    )
    expect(fresh.getCss(null, false)).toContain('green')
    expect(fresh.getCss(null, false)).not.toContain('blue')
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

    expect(
      requireFromPlugin<{ isolated: boolean }>('@devup-ui/wasm', root),
    ).toEqual({ isolated: true })
  })

  it('loads or injects the Webpack plugin without a static dependency', () => {
    expect(typeof loadWebpackPlugin().DevupUIWebpackPlugin).toBe('function')
    setWebpackPluginForTesting(webpackPlugin)
    expect(loadWebpackPlugin()).toBe(webpackPlugin)
  })
})
