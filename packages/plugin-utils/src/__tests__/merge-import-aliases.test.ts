import { describe, expect, it } from 'bun:test'

import {
  DEFAULT_IMPORT_ALIASES,
  mergeImportAliases,
  type WasmImportAliases,
} from '../types'

describe('mergeImportAliases', () => {
  it('should return default aliases when no user aliases provided', () => {
    const result = mergeImportAliases()

    expect(result).toEqual({
      '@emotion/styled': 'styled',
      'styled-components': 'styled',
      '@vanilla-extract/css': null,
    })
  })

  it('should merge user aliases with defaults', () => {
    const result = mergeImportAliases({
      'my-lib': 'customExport',
    })

    expect(result).toEqual({
      '@emotion/styled': 'styled',
      'styled-components': 'styled',
      '@vanilla-extract/css': null,
      'my-lib': 'customExport',
    })
  })

  it('should allow user to override default aliases', () => {
    const result = mergeImportAliases({
      '@emotion/styled': 'customStyled',
    })

    expect(result['@emotion/styled']).toBe('customStyled')
  })

  it('should allow user to disable default aliases with false', () => {
    const result = mergeImportAliases({
      '@emotion/styled': false,
    })

    expect(result['@emotion/styled']).toBeUndefined()
    expect(result['styled-components']).toBe('styled')
  })

  it('should convert true to null for named exports', () => {
    const result = mergeImportAliases({
      'my-lib': true,
    })

    expect(result['my-lib']).toBeNull()
  })

  it('should handle disabling all defaults', () => {
    const result = mergeImportAliases({
      '@emotion/styled': false,
      'styled-components': false,
      '@vanilla-extract/css': false,
    })

    expect(result).toEqual({})
  })

  it('should return correct WASM format', () => {
    const result: WasmImportAliases = mergeImportAliases({
      'default-export-lib': 'namedExport',
      'named-export-lib': true,
    })

    // String value = default export -> named export
    expect(result['default-export-lib']).toBe('namedExport')
    // null = named exports 1:1 mapping
    expect(result['named-export-lib']).toBeNull()
  })
})

describe('DEFAULT_IMPORT_ALIASES', () => {
  it('should have correct default values', () => {
    expect(DEFAULT_IMPORT_ALIASES).toEqual({
      '@emotion/styled': 'styled',
      'styled-components': 'styled',
      '@vanilla-extract/css': true,
    })
  })
})
