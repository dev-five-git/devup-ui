import * as nodePath from 'node:path'

import * as wasm from '@devup-ui/wasm'
import {
  afterEach,
  beforeEach,
  describe,
  expect,
  it,
  mock,
  spyOn,
} from 'bun:test'

import devupUICssLoader from '../css-loader'

let resolveSpy: ReturnType<typeof spyOn>
let getCssSpy: ReturnType<typeof spyOn>
let codeExtractSpy: ReturnType<typeof spyOn>
let registerThemeSpy: ReturnType<typeof spyOn>
let setDebugSpy: ReturnType<typeof spyOn>

beforeEach(() => {
  resolveSpy = spyOn(nodePath, 'resolve').mockReturnValue('resolved')
  getCssSpy = spyOn(wasm, 'getCss').mockReturnValue('get css')
  codeExtractSpy = spyOn(wasm, 'codeExtract').mockImplementation(
    (_path: string, contents: string) =>
      ({
        css: '',
        code: contents,
        cssFile: '',
        map: undefined,
        updatedBaseStyle: false,
        free: mock(),
        [Symbol.dispose]: mock(),
      }) as any,
  )
  registerThemeSpy = spyOn(wasm, 'registerTheme').mockReturnValue(undefined)
  setDebugSpy = spyOn(wasm, 'setDebug').mockReturnValue(undefined)
})

afterEach(() => {
  resolveSpy.mockRestore()
  getCssSpy.mockRestore()
  codeExtractSpy.mockRestore()
  registerThemeSpy.mockRestore()
  setDebugSpy.mockRestore()
})

describe('devupUICssLoader', () => {
  it('should return css on no watch', () => {
    const callback = mock()
    const addContextDependency = mock()
    resolveSpy.mockReturnValue('resolved')
    getCssSpy.mockReturnValue('get css')
    devupUICssLoader.bind({
      callback,
      addContextDependency,
      resourcePath: 'devup-ui.css',
      getOptions: () => ({ watch: false }),
    } as any)(Buffer.from('data'), '')
    expect(callback).toBeCalledWith(null, 'get css', '', undefined)
  })

  it('should return _compiler hit css on watch', () => {
    const callback = mock()
    const addContextDependency = mock()
    resolveSpy.mockReturnValue('resolved')
    getCssSpy.mockReturnValue('get css')
    devupUICssLoader.bind({
      callback,
      addContextDependency,
      getOptions: () => ({ watch: true }),
      resourcePath: 'devup-ui.css',
    } as any)(Buffer.from('data'), '')
    expect(callback).toBeCalledWith(null, 'get css', '', undefined)
    expect(getCssSpy).toBeCalledTimes(1)
    getCssSpy.mockClear()
    devupUICssLoader.bind({
      callback,
      addContextDependency,
      getOptions: () => ({ watch: true }),
      resourcePath: 'devup-ui.css',
    } as any)(Buffer.from('data'), '')

    expect(getCssSpy).toBeCalledTimes(1)
    getCssSpy.mockClear()

    devupUICssLoader.bind({
      callback,
      addContextDependency,
      getOptions: () => ({ watch: true }),
      resourcePath: 'devup-ui-10.css',
    } as any)(Buffer.from(''), '')

    expect(getCssSpy).toBeCalledTimes(1)
  })
})
