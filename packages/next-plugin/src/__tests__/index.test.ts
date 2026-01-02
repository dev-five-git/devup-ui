import * as wasm from '@devup-ui/wasm'
import * as webpackPluginModule from '@devup-ui/webpack-plugin'
import {
  afterAll,
  beforeAll,
  describe,
  expect,
  it,
  mock,
  spyOn,
} from 'bun:test'

let exportClassMapSpy: ReturnType<typeof spyOn>
let exportFileMapSpy: ReturnType<typeof spyOn>
let exportSheetSpy: ReturnType<typeof spyOn>
let getDefaultThemeSpy: ReturnType<typeof spyOn>
let getThemeInterfaceSpy: ReturnType<typeof spyOn>
let devupUIWebpackPluginSpy: ReturnType<typeof spyOn>

beforeAll(() => {
  exportClassMapSpy = spyOn(wasm, 'exportClassMap').mockReturnValue('{}')
  exportFileMapSpy = spyOn(wasm, 'exportFileMap').mockReturnValue('{}')
  exportSheetSpy = spyOn(wasm, 'exportSheet').mockReturnValue('{}')
  getDefaultThemeSpy = spyOn(wasm, 'getDefaultTheme').mockReturnValue(undefined)
  getThemeInterfaceSpy = spyOn(wasm, 'getThemeInterface').mockReturnValue('')
  devupUIWebpackPluginSpy = spyOn(
    webpackPluginModule,
    'DevupUIWebpackPlugin',
  ).mockImplementation(mock() as never)
})

afterAll(() => {
  exportClassMapSpy.mockRestore()
  exportFileMapSpy.mockRestore()
  exportSheetSpy.mockRestore()
  getDefaultThemeSpy.mockRestore()
  getThemeInterfaceSpy.mockRestore()
  devupUIWebpackPluginSpy.mockRestore()
})

describe('export', () => {
  it('should export DevupUI', async () => {
    const index = await import('../index')
    expect({ ...index }).toEqual({
      DevupUI: expect.any(Function),
    })
  })
})
