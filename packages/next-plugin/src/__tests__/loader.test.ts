import { join } from 'node:path'

import type { Mock } from 'bun:test'
import {
  afterEach,
  beforeEach,
  describe,
  expect,
  it,
  mock,
  spyOn,
} from 'bun:test'

const mockExistsSync = mock((_path: string) => false)
const mockReadFileSync = mock((_path: string, _encoding?: string) => '{}')

mock.module('node:fs', () => ({
  existsSync: mockExistsSync,
  readFileSync: mockReadFileSync,
}))

mock.module('node:fs/promises', () => ({
  writeFile: mock(),
}))

mock.module('@devup-ui/wasm', () => ({
  codeExtract: mock(),
  exportClassMap: mock(),
  exportFileMap: mock(),
  exportSheet: mock(),
  getCss: mock(),
  importClassMap: mock(),
  importFileMap: mock(),
  importSheet: mock(),
  registerTheme: mock(),
}))

import { writeFile } from 'node:fs/promises'

import {
  codeExtract,
  exportClassMap,
  exportFileMap,
  exportSheet,
  getCss,
  importClassMap,
  importFileMap,
  importSheet,
  registerTheme,
} from '@devup-ui/wasm'

import devupUILoader from '../loader'

let dateNowSpy: ReturnType<typeof spyOn>

beforeEach(() => {
  mockExistsSync.mockReset()
  mockReadFileSync.mockReset()
  ;(codeExtract as Mock<typeof codeExtract>).mockReset()
  ;(exportClassMap as Mock<typeof exportClassMap>).mockReset()
  ;(exportFileMap as Mock<typeof exportFileMap>).mockReset()
  ;(exportSheet as Mock<typeof exportSheet>).mockReset()
  ;(getCss as Mock<typeof getCss>).mockReset()
  ;(importClassMap as Mock<typeof importClassMap>).mockReset()
  ;(importFileMap as Mock<typeof importFileMap>).mockReset()
  ;(importSheet as Mock<typeof importSheet>).mockReset()
  ;(registerTheme as Mock<typeof registerTheme>).mockReset()
  ;(writeFile as Mock<typeof writeFile>).mockReset()

  mockExistsSync.mockReturnValue(false)
  mockReadFileSync.mockReturnValue('{}')

  dateNowSpy = spyOn(Date, 'now').mockReturnValue(0)
})

afterEach(() => {
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
    ;(exportSheet as Mock<typeof exportSheet>).mockReturnValue('sheet')
    ;(exportClassMap as Mock<typeof exportClassMap>).mockReturnValue('classMap')
    ;(exportFileMap as Mock<typeof exportFileMap>).mockReturnValue('fileMap')
    ;(getCss as Mock<typeof getCss>).mockReturnValue('css')
    ;(codeExtract as Mock<typeof codeExtract>).mockReturnValue({
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
    expect(codeExtract).toHaveBeenCalledWith(
      'index.tsx',
      'code',
      'package',
      './cssFile',
      true,
      false,
      true,
    )
    if (options.updatedBaseStyle) {
      expect(writeFile).toHaveBeenCalledWith(
        join('cssFile', 'devup-ui.css'),
        'css',
        'utf-8',
      )
    } else {
      expect(writeFile).not.toHaveBeenCalledWith(
        join('cssFile', 'devup-ui.css'),
        'css',
        'utf-8',
      )
    }
    await waitFor(() => {
      expect(asyncCallback).toHaveBeenCalledWith(null, 'code', {})
      expect(writeFile).toHaveBeenCalledWith(
        join('cssFile', 'cssFile'),
        '/* index.tsx 0 */',
      )
      expect(writeFile).toHaveBeenCalledWith('sheetFile', 'sheet')
      expect(writeFile).toHaveBeenCalledWith('classMapFile', 'classMap')
      expect(writeFile).toHaveBeenCalledWith('fileMapFile', 'fileMap')
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
    ;(codeExtract as Mock<typeof codeExtract>).mockReturnValue({
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
    expect(codeExtract).toHaveBeenCalledWith(
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
    expect(writeFile).not.toHaveBeenCalledWith('cssFile', 'css', {
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
    ;(codeExtract as Mock<typeof codeExtract>).mockImplementation(() => {
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
    ;(exportSheet as Mock<typeof exportSheet>).mockReturnValue('sheet')
    ;(exportClassMap as Mock<typeof exportClassMap>).mockReturnValue('classMap')
    ;(exportFileMap as Mock<typeof exportFileMap>).mockReturnValue('fileMap')
    ;(codeExtract as Mock<typeof codeExtract>).mockReturnValue({
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
    expect(codeExtract).toHaveBeenCalledWith(
      'index.tsx',
      'code',
      'package',
      './cssFile',
      true,
      false,
      true,
    )
    await waitFor(() => {
      expect(writeFile).toHaveBeenCalledWith(
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
    ;(codeExtract as Mock<typeof codeExtract>).mockReturnValue({
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
    ;(registerTheme as Mock<typeof registerTheme>).mockReturnValueOnce(
      undefined,
    )
    ;(codeExtract as Mock<typeof codeExtract>).mockReturnValue({
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
