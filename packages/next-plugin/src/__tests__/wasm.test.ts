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
  beforeEach,
  describe,
  expect,
  it,
  mock,
} from 'bun:test'

import {
  createWasm,
  loadWasm,
  loadWebpackPlugin,
  requireFromPlugin,
  setWasmForTesting,
  setWebpackPluginForTesting,
  withModuleResolver,
} from '../wasm'

const originalCwd = process.cwd()
let tempRoots: string[] = []

beforeAll(() => {
  tempRoots = []
})

beforeEach(() => {
  setWasmForTesting(undefined)
  setWebpackPluginForTesting(undefined)
})

afterEach(() => {
  process.chdir(originalCwd)
  setWasmForTesting(undefined)
  setWebpackPluginForTesting(undefined)
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
    const otherRoot = mkdtempSync(join(tmpdir(), 'devup-ui-resolver-cwd-'))
    tempRoots.push(otherRoot)
    process.chdir(otherRoot)
    expect(resolveModule('./wasm.test', import.meta.path)?.path).toBe(
      relative(originalCwd, import.meta.path).replaceAll('\\', '/'),
    )
    const older = {} as typeof wasm
    expect(withModuleResolver(older)).toBe(older)
  })

  it('uses an injected namespace in tests', () => {
    setWasmForTesting(wasm)
    expect(loadWasm()).toBe(wasm)
    expect(createWasm()).toBe(wasm)
    expect(createWasm('/unused-injected-root')).toBe(wasm)
  })

  it('loads the package once', () => {
    const loaded = loadWasm()
    expect(typeof loaded.codeExtract).toBe('function')
    expect(loadWasm()).toBe(loaded)
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

  it('loads or injects the Webpack plugin without a static dependency', () => {
    expect(typeof loadWebpackPlugin().DevupUIWebpackPlugin).toBe('function')
    setWebpackPluginForTesting(webpackPlugin)
    expect(loadWebpackPlugin()).toBe(webpackPlugin)
  })
})
