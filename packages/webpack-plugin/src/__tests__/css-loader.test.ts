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

type CodeExtractResult = ReturnType<typeof wasm.codeExtract>
type CssLoaderThis = ThisParameterType<typeof devupUICssLoader>

function createCodeExtractResult(
  overrides: Partial<CodeExtractResult> = {},
): CodeExtractResult {
  return {
    css: '',
    code: '',
    cssFile: '',
    map: undefined,
    updatedBaseStyle: false,
    free: mock(),
    [Symbol.dispose]: mock(),
    ...overrides,
  } as unknown as CodeExtractResult
}

function createCssLoaderContext(
  resourcePath: string,
  callback: ReturnType<typeof mock>,
): CssLoaderThis {
  return {
    callback,
    addContextDependency: mock(),
    resourcePath,
    getOptions: () => ({ watch: resourcePath.includes('devup-ui') }),
  } as unknown as CssLoaderThis
}

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
      createCodeExtractResult({
        code: contents,
      }),
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
    resolveSpy.mockReturnValue('resolved')
    getCssSpy.mockReturnValue('get css')
    devupUICssLoader.bind(createCssLoaderContext('devup-ui.css', callback))(
      Buffer.from('data'),
      '',
    )
    expect(callback).toBeCalledWith(null, 'get css', '', undefined)
  })

  it('should return _compiler hit css on watch', () => {
    const callback = mock()
    resolveSpy.mockReturnValue('resolved')
    getCssSpy.mockReturnValue('get css')
    devupUICssLoader.bind(createCssLoaderContext('devup-ui.css', callback))(
      Buffer.from('data'),
      '',
    )
    expect(callback).toBeCalledWith(null, 'get css', '', undefined)
    expect(getCssSpy).toBeCalledTimes(1)
    getCssSpy.mockClear()
    devupUICssLoader.bind(createCssLoaderContext('devup-ui.css', callback))(
      Buffer.from('data'),
      '',
    )

    expect(getCssSpy).toBeCalledTimes(1)
    getCssSpy.mockClear()

    devupUICssLoader.bind(createCssLoaderContext('devup-ui-10.css', callback))(
      Buffer.from(''),
      '',
    )

    expect(getCssSpy).toBeCalledTimes(1)
  })
})
