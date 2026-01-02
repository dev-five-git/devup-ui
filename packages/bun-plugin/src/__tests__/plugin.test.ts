import { getDefaultTheme } from '@devup-ui/wasm'
import { beforeEach, describe, expect, it, mock } from 'bun:test'

import { getDevupDefaultTheme, getDevupDefine } from '../plugin'

mock.module('@devup-ui/wasm', () => ({
  getDefaultTheme: mock(),
  getThemeInterface: mock(),
  getCss: mock(),
  importClassMap: mock(),
  importFileMap: mock(),
  importSheet: mock(),
  registerTheme: mock(),
}))

const mockedGetDefaultTheme = getDefaultTheme as ReturnType<typeof mock>

beforeEach(() => {
  mockedGetDefaultTheme.mockReset()
  mockedGetDefaultTheme.mockReturnValue('default')
})

describe('getDevupDefaultTheme', () => {
  it('should return theme from WASM', async () => {
    mockedGetDefaultTheme.mockReturnValue('dark')
    expect(await getDevupDefaultTheme()).toBe('dark')
  })
})

describe('getDevupDefine', () => {
  it('should return define object with theme', async () => {
    mockedGetDefaultTheme.mockReturnValue('dark')
    const define = await getDevupDefine()
    expect(define['process.env.DEVUP_UI_DEFAULT_THEME']).toBe('"dark"')
  })

  it('should return empty object when no theme', async () => {
    mockedGetDefaultTheme.mockReturnValue('')
    const define = getDevupDefine()
    expect(define).toEqual({})
  })
})
