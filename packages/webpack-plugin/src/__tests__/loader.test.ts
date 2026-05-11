import * as fsPromises from 'node:fs/promises'
import { join } from 'node:path'

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

import type { DevupUILoaderOptions } from '../loader'
import devupUILoader from '../loader'

type CodeExtractResult = ReturnType<typeof wasm.codeExtract>
type LoaderThis = ThisParameterType<typeof devupUILoader>
type LoaderCallback = ReturnType<LoaderThis['async']>
interface TestLoaderContext extends Pick<
  LoaderThis,
  'async' | 'resourcePath' | 'addDependency'
> {
  getOptions: () => Partial<DevupUILoaderOptions>
  _compiler?: { __DEVUP_CACHE: string }
}

function createCodeExtractResult(
  overrides: Partial<CodeExtractResult> = {},
): CodeExtractResult {
  return {
    code: '',
    css: '',
    cssFile: undefined,
    updatedBaseStyle: false,
    map: undefined,
    free: () => {},
    [Symbol.dispose]: () => {},
    ...overrides,
  } as unknown as CodeExtractResult
}

function createLoaderContext(
  options: Partial<DevupUILoaderOptions>,
  callback: LoaderCallback,
  resourcePath = 'index.tsx',
  overrides: Partial<TestLoaderContext> = {},
): LoaderThis {
  return {
    getOptions: () => options,
    async: mock().mockReturnValue(callback),
    resourcePath,
    addDependency: mock(),
    ...overrides,
  } as unknown as LoaderThis
}

let codeExtractSpy: ReturnType<typeof spyOn>
let exportClassMapSpy: ReturnType<typeof spyOn>
let exportFileMapSpy: ReturnType<typeof spyOn>
let exportSheetSpy: ReturnType<typeof spyOn>
let getCssSpy: ReturnType<typeof spyOn>
let getDefaultThemeSpy: ReturnType<typeof spyOn>
let getThemeInterfaceSpy: ReturnType<typeof spyOn>
let importClassMapSpy: ReturnType<typeof spyOn>
let importFileMapSpy: ReturnType<typeof spyOn>
let importSheetSpy: ReturnType<typeof spyOn>
let registerThemeSpy: ReturnType<typeof spyOn>
let setDebugSpy: ReturnType<typeof spyOn>
let setPrefixSpy: ReturnType<typeof spyOn>
let writeFileSpy: ReturnType<typeof spyOn>
let dateNowSpy: ReturnType<typeof spyOn>

beforeEach(() => {
  codeExtractSpy = spyOn(wasm, 'codeExtract').mockReturnValue(
    createCodeExtractResult(),
  )
  exportClassMapSpy = spyOn(wasm, 'exportClassMap').mockReturnValue('{}')
  exportFileMapSpy = spyOn(wasm, 'exportFileMap').mockReturnValue('{}')
  exportSheetSpy = spyOn(wasm, 'exportSheet').mockReturnValue('{}')
  getCssSpy = spyOn(wasm, 'getCss').mockReturnValue('')
  getDefaultThemeSpy = spyOn(wasm, 'getDefaultTheme').mockReturnValue(undefined)
  getThemeInterfaceSpy = spyOn(wasm, 'getThemeInterface').mockReturnValue('')
  importClassMapSpy = spyOn(wasm, 'importClassMap').mockReturnValue(undefined)
  importFileMapSpy = spyOn(wasm, 'importFileMap').mockReturnValue(undefined)
  importSheetSpy = spyOn(wasm, 'importSheet').mockReturnValue(undefined)
  registerThemeSpy = spyOn(wasm, 'registerTheme').mockReturnValue(undefined)
  setDebugSpy = spyOn(wasm, 'setDebug').mockReturnValue(undefined)
  setPrefixSpy = spyOn(wasm, 'setPrefix').mockReturnValue(undefined)
  writeFileSpy = spyOn(fsPromises, 'writeFile').mockResolvedValue(undefined)
  dateNowSpy = spyOn(Date, 'now').mockReturnValue(0)
})

afterEach(() => {
  codeExtractSpy.mockRestore()
  exportClassMapSpy.mockRestore()
  exportFileMapSpy.mockRestore()
  exportSheetSpy.mockRestore()
  getCssSpy.mockRestore()
  getDefaultThemeSpy.mockRestore()
  getThemeInterfaceSpy.mockRestore()
  importClassMapSpy.mockRestore()
  importFileMapSpy.mockRestore()
  importSheetSpy.mockRestore()
  registerThemeSpy.mockRestore()
  setDebugSpy.mockRestore()
  setPrefixSpy.mockRestore()
  writeFileSpy.mockRestore()
  dateNowSpy.mockRestore()
})

const waitFor = async (fn: () => void, timeout = 1000) => {
  const start = performance.now()
  while (performance.now() - start < timeout) {
    try {
      fn()
      return
    } catch {
      await new Promise((r) => setTimeout(r, 10))
    }
  }
  fn()
}

describe('devupUILoader', () => {
  it.each(
    createTestMatrix({
      updatedBaseStyle: [true, false],
    }),
  )('should extract code with css', async (options) => {
    const _compiler = {
      __DEVUP_CACHE: '',
    }
    const asyncCallback = mock()
    const t = createLoaderContext(
      {
        package: 'package',
        cssDir: 'cssFile',
        sheetFile: 'sheetFile',
        classMapFile: 'classMapFile',
        fileMapFile: 'fileMapFile',
        watch: true,
        singleCss: true,
      },
      asyncCallback,
      'index.tsx',
      { _compiler },
    )
    exportSheetSpy.mockReturnValue('sheet')
    exportClassMapSpy.mockReturnValue('classMap')
    exportFileMapSpy.mockReturnValue('fileMap')
    getCssSpy.mockReturnValue('css')
    codeExtractSpy.mockReturnValue(
      createCodeExtractResult({
        code: 'code',
        css: 'css',
        map: '{}',
        cssFile: 'cssFile',
        updatedBaseStyle: options.updatedBaseStyle,
      }),
    )
    devupUILoader.bind(t)(Buffer.from('code'), 'index.tsx')

    expect(t.async).toHaveBeenCalled()
    expect(codeExtractSpy).toHaveBeenCalledWith(
      'index.tsx',
      'code',
      'package',
      './cssFile',
      true,
      false,
      true,
      {},
    )
    if (options.updatedBaseStyle) {
      expect(writeFileSpy).toHaveBeenCalledWith(
        join('cssFile', 'devup-ui.css'),
        'css',
        'utf-8',
      )
    } else {
      expect(writeFileSpy).not.toHaveBeenCalledWith(
        join('cssFile', 'devup-ui.css'),
        'css',
        'utf-8',
      )
    }
    await waitFor(() => {
      expect(asyncCallback).toHaveBeenCalledWith(null, 'code', {})
      expect(writeFileSpy).toHaveBeenCalledWith(
        join('cssFile', 'cssFile'),
        '/* index.tsx 0 */',
      )
      expect(writeFileSpy).toHaveBeenCalledWith('sheetFile', 'sheet')
      expect(writeFileSpy).toHaveBeenCalledWith('classMapFile', 'classMap')
      expect(writeFileSpy).toHaveBeenCalledWith('fileMapFile', 'fileMap')
    })
  })

  it('should extract code without css', async () => {
    const asyncCallback = mock()
    const t = createLoaderContext(
      {
        package: 'package',
        cssDir: 'cssFile',
        watch: false,
        singleCss: true,
      },
      asyncCallback,
    )
    codeExtractSpy.mockReturnValue(
      createCodeExtractResult({
        code: 'code',
        css: undefined,
        map: undefined,
        cssFile: undefined,
        updatedBaseStyle: false,
      }),
    )
    devupUILoader.bind(t)(Buffer.from('code'), 'index.tsx')

    expect(t.async).toHaveBeenCalled()
    expect(codeExtractSpy).toHaveBeenCalledWith(
      'index.tsx',
      'code',
      'package',
      './cssFile',
      true,
      false,
      true,
      {},
    )
    await waitFor(() => {
      expect(asyncCallback).toHaveBeenCalledWith(null, 'code', null)
    })
    expect(writeFileSpy).not.toHaveBeenCalledWith('cssFile', 'css', {
      encoding: 'utf-8',
    })
  })

  it('should handle error', async () => {
    const asyncCallback = mock()
    const t = createLoaderContext(
      {
        package: 'package',
        cssDir: 'cssFile',
        watch: false,
        singleCss: true,
      },
      asyncCallback,
    )
    codeExtractSpy.mockImplementation(() => {
      throw new Error('error')
    })
    devupUILoader.bind(t)(Buffer.from('code'), 'index.tsx')

    expect(t.async).toHaveBeenCalled()
    expect(asyncCallback).toHaveBeenCalledWith(new Error('error'))
  })

  it('should load with date now on watch', async () => {
    const asyncCallback = mock()
    const t = createLoaderContext(
      {
        package: 'package',
        cssDir: 'cssFile',
        watch: true,
        singleCss: true,
      },
      asyncCallback,
    )
    codeExtractSpy.mockReturnValue(
      createCodeExtractResult({
        code: 'code',
        css: 'css',
        map: undefined,
        cssFile: 'cssFile',
        updatedBaseStyle: false,
      }),
    )
    devupUILoader.bind(t)(Buffer.from('code'), 'index.tsx')

    expect(t.async).toHaveBeenCalled()
    expect(codeExtractSpy).toHaveBeenCalledWith(
      'index.tsx',
      'code',
      'package',
      './cssFile',
      true,
      false,
      true,
      {},
    )
  })

  it('should load with nowatch', async () => {
    const asyncCallback = mock()
    const t = createLoaderContext(
      {
        package: 'package',
        cssDir: './foo',
        watch: false,
        singleCss: true,
      },
      asyncCallback,
      './foo/index.tsx',
    )
    codeExtractSpy.mockReturnValue(
      createCodeExtractResult({
        code: 'code',
        css: 'css',
        map: undefined,
        cssFile: 'cssFile',
        updatedBaseStyle: false,
      }),
    )
    devupUILoader.bind(t)(Buffer.from('code'), '/foo/index.tsx')
  })
  it('should load with theme', async () => {
    const asyncCallback = mock()
    const t = createLoaderContext(
      {
        package: 'package',
        cssDir: 'cssFile',
        watch: false,
        singleCss: true,
        theme: {
          colors: {
            primary: '#000',
          },
        },
      },
      asyncCallback,
    )
    registerThemeSpy.mockReturnValueOnce(undefined)
    codeExtractSpy.mockReturnValue(
      createCodeExtractResult({
        code: 'code',
        css: 'css',
        map: undefined,
        cssFile: 'cssFile',
        updatedBaseStyle: false,
      }),
    )
    devupUILoader.bind(t)(Buffer.from('code'), 'index.tsx')
  })
})
