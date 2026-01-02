import * as fs from 'node:fs'
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

import devupUILoader from '../loader'

let existsSyncSpy: ReturnType<typeof spyOn>
let readFileSyncSpy: ReturnType<typeof spyOn>
let writeFileSpy: ReturnType<typeof spyOn>
let codeExtractSpy: ReturnType<typeof spyOn>
let exportClassMapSpy: ReturnType<typeof spyOn>
let exportFileMapSpy: ReturnType<typeof spyOn>
let exportSheetSpy: ReturnType<typeof spyOn>
let getCssSpy: ReturnType<typeof spyOn>
let importClassMapSpy: ReturnType<typeof spyOn>
let importFileMapSpy: ReturnType<typeof spyOn>
let importSheetSpy: ReturnType<typeof spyOn>
let registerThemeSpy: ReturnType<typeof spyOn>
let dateNowSpy: ReturnType<typeof spyOn>

beforeEach(() => {
  existsSyncSpy = spyOn(fs, 'existsSync').mockReturnValue(false)
  readFileSyncSpy = spyOn(fs, 'readFileSync').mockReturnValue('{}')
  writeFileSpy = spyOn(fsPromises, 'writeFile').mockResolvedValue(undefined)
  codeExtractSpy = spyOn(wasm, 'codeExtract')
  exportClassMapSpy = spyOn(wasm, 'exportClassMap')
  exportFileMapSpy = spyOn(wasm, 'exportFileMap')
  exportSheetSpy = spyOn(wasm, 'exportSheet')
  getCssSpy = spyOn(wasm, 'getCss')
  importClassMapSpy = spyOn(wasm, 'importClassMap')
  importFileMapSpy = spyOn(wasm, 'importFileMap')
  importSheetSpy = spyOn(wasm, 'importSheet')
  registerThemeSpy = spyOn(wasm, 'registerTheme')
  dateNowSpy = spyOn(Date, 'now').mockReturnValue(0)
})

afterEach(() => {
  existsSyncSpy.mockRestore()
  readFileSyncSpy.mockRestore()
  writeFileSpy.mockRestore()
  codeExtractSpy.mockRestore()
  exportClassMapSpy.mockRestore()
  exportFileMapSpy.mockRestore()
  exportSheetSpy.mockRestore()
  getCssSpy.mockRestore()
  importClassMapSpy.mockRestore()
  importFileMapSpy.mockRestore()
  importSheetSpy.mockRestore()
  registerThemeSpy.mockRestore()
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
    const t = {
      getOptions: () => ({
        package: 'package',
        cssDir: 'cssFile',
        sheetFile: 'sheetFile',
        classMapFile: 'classMapFile',
        fileMapFile: 'fileMapFile',
        themeFile: 'themeFile',
        watch: true,
        singleCss: true,
      }),
      async: mock().mockReturnValue(asyncCallback),
      resourcePath: 'index.tsx',
      addDependency: mock(),
      _compiler,
    }
    exportSheetSpy.mockReturnValue('sheet')
    exportClassMapSpy.mockReturnValue('classMap')
    exportFileMapSpy.mockReturnValue('fileMap')
    getCssSpy.mockReturnValue('css')
    codeExtractSpy.mockReturnValue({
      code: 'code',
      css: 'css',
      free: mock(),
      map: '{}',
      cssFile: 'cssFile',
      updatedBaseStyle: options.updatedBaseStyle,
      [Symbol.dispose]: mock(),
    })
    devupUILoader.bind(t as any)(Buffer.from('code'), 'index.tsx')

    expect(t.async).toHaveBeenCalled()
    expect(codeExtractSpy).toHaveBeenCalledWith(
      'index.tsx',
      'code',
      'package',
      './cssFile',
      true,
      false,
      true,
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
    const t = {
      getOptions: () => ({
        package: 'package',
        cssDir: 'cssFile',
        watch: false,
        singleCss: true,
        defaultClassMap: {},
        defaultFileMap: {},
        defaultSheet: {},
      }),
      async: mock().mockReturnValue(asyncCallback),
      resourcePath: 'index.tsx',
      addDependency: mock(),
    }
    codeExtractSpy.mockReturnValue({
      code: 'code',
      css: undefined,
      free: mock(),
      map: undefined,
      cssFile: undefined,
      updatedBaseStyle: false,
      [Symbol.dispose]: mock(),
    })
    devupUILoader.bind(t as any)(Buffer.from('code'), 'index.tsx')

    expect(t.async).toHaveBeenCalled()
    expect(codeExtractSpy).toHaveBeenCalledWith(
      'index.tsx',
      'code',
      'package',
      './cssFile',
      true,
      false,
      true,
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
    const t = {
      getOptions: () => ({
        package: 'package',
        cssDir: 'cssFile',
        watch: false,
        singleCss: true,
        defaultClassMap: {},
        defaultFileMap: {},
        defaultSheet: {},
      }),
      async: mock().mockReturnValue(asyncCallback),
      resourcePath: 'index.tsx',
      addDependency: mock(),
    }
    codeExtractSpy.mockImplementation(() => {
      throw new Error('error')
    })
    devupUILoader.bind(t as any)(Buffer.from('code'), 'index.tsx')

    expect(t.async).toHaveBeenCalled()
    await waitFor(() => {
      expect(asyncCallback).toHaveBeenCalledWith(new Error('error'))
    })
  })

  it('should load with date now on watch', async () => {
    const asyncCallback = mock()
    const t = {
      getOptions: () => ({
        package: 'package',
        cssDir: 'cssFile',
        sheetFile: 'sheetFile',
        classMapFile: 'classMapFile',
        fileMapFile: 'fileMapFile',
        themeFile: 'themeFile',
        watch: true,
        singleCss: true,
      }),
      async: mock().mockReturnValue(asyncCallback),
      resourcePath: 'index.tsx',
      addDependency: mock(),
    }
    exportSheetSpy.mockReturnValue('sheet')
    exportClassMapSpy.mockReturnValue('classMap')
    exportFileMapSpy.mockReturnValue('fileMap')
    codeExtractSpy.mockReturnValue({
      code: 'code',
      css: 'css',
      free: mock(),
      map: undefined,
      cssFile: 'cssFile',
      updatedBaseStyle: false,
      [Symbol.dispose]: mock(),
    })
    devupUILoader.bind(t as any)(Buffer.from('code'), 'index.tsx')

    expect(t.async).toHaveBeenCalled()
    expect(codeExtractSpy).toHaveBeenCalledWith(
      'index.tsx',
      'code',
      'package',
      './cssFile',
      true,
      false,
      true,
    )
    await waitFor(() => {
      expect(writeFileSpy).toHaveBeenCalledWith(
        join('cssFile', 'cssFile'),
        '/* index.tsx 0 */',
      )
    })
  })

  it('should load with nowatch', async () => {
    const asyncCallback = mock()
    const t = {
      getOptions: () => ({
        package: 'package',
        cssDir: './foo',
        watch: false,
        singleCss: true,
        defaultClassMap: {},
        defaultFileMap: {},
        defaultSheet: {},
      }),
      async: mock().mockReturnValue(asyncCallback),
      resourcePath: './foo/index.tsx',
      addDependency: mock(),
    }
    codeExtractSpy.mockReturnValue({
      code: 'code',
      css: 'css',
      free: mock(),
      map: undefined,
      cssFile: 'cssFile',
      updatedBaseStyle: false,
      [Symbol.dispose]: mock(),
    })
    devupUILoader.bind(t as any)(Buffer.from('code'), '/foo/index.tsx')
    await waitFor(() => {
      expect(asyncCallback).toHaveBeenCalledWith(null, 'code', null)
    })
  })

  it('should load with theme', async () => {
    const asyncCallback = mock()
    const t = {
      getOptions: () => ({
        package: 'package',
        cssDir: 'cssFile',
        watch: false,
        singleCss: true,
        theme: {
          colors: {
            primary: '#000',
          },
        },
        defaultClassMap: {},
        defaultFileMap: {},
        defaultSheet: {},
      }),
      async: mock().mockReturnValue(asyncCallback),
      resourcePath: 'index.tsx',
      addDependency: mock(),
    }
    registerThemeSpy.mockReturnValueOnce(undefined)
    codeExtractSpy.mockReturnValue({
      code: 'code',
      css: 'css',
      free: mock(),
      map: undefined,
      cssFile: 'cssFile',
      updatedBaseStyle: false,
      [Symbol.dispose]: mock(),
    })
    devupUILoader.bind(t as any)(Buffer.from('code'), 'index.tsx')
    await waitFor(() => {
      expect(asyncCallback).toHaveBeenCalledWith(null, 'code', null)
    })
  })
})
