import * as wasm from '@devup-ui/wasm'
import * as webpackPlugin from '@devup-ui/webpack-plugin'
import { afterEach, describe, expect, it } from 'bun:test'

import {
  loadWasm,
  loadWebpackPlugin,
  requireWasm,
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
