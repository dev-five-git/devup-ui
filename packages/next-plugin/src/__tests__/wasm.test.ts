import * as wasm from '@devup-ui/wasm'
import { afterEach, describe, expect, it } from 'bun:test'

import { loadWasm, setWasmForTesting } from '../wasm'

afterEach(() => setWasmForTesting(undefined))

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
})
