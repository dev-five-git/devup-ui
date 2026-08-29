import * as wasm from '@devup-ui/wasm'
import * as webpackPlugin from '@devup-ui/webpack-plugin'
import { afterEach, describe, expect, it } from 'bun:test'

import {
  loadWasm,
  loadWebpackPlugin,
  setWasmForTesting,
  setWebpackPluginForTesting,
} from '../wasm'

afterEach(() => {
  setWasmForTesting(undefined)
  setWebpackPluginForTesting(undefined)
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

  it('loads or injects the Webpack plugin without a static dependency', () => {
    expect(typeof loadWebpackPlugin().DevupUIWebpackPlugin).toBe('function')
    setWebpackPluginForTesting(webpackPlugin)
    expect(loadWebpackPlugin()).toBe(webpackPlugin)
  })
})
