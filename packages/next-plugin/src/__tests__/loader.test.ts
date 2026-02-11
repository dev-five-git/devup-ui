import * as fs from 'node:fs'
import * as fsPromises from 'node:fs/promises'
import * as http from 'node:http'
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

import devupUILoader, { resetInit } from '../loader'

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
  resetInit()
  existsSyncSpy = spyOn(fs, 'existsSync').mockReturnValue(false)
  readFileSyncSpy = spyOn(fs, 'readFileSync').mockReturnValue('{}')
  writeFileSpy = spyOn(fsPromises, 'writeFile').mockResolvedValue(undefined)
  codeExtractSpy = spyOn(wasm, 'codeExtract')
  exportClassMapSpy = spyOn(wasm, 'exportClassMap')
  exportFileMapSpy = spyOn(wasm, 'exportFileMap')
  exportSheetSpy = spyOn(wasm, 'exportSheet')
  getCssSpy = spyOn(wasm, 'getCss')
  importClassMapSpy = spyOn(wasm, 'importClassMap').mockImplementation(() => {})
  importFileMapSpy = spyOn(wasm, 'importFileMap').mockImplementation(() => {})
  importSheetSpy = spyOn(wasm, 'importSheet').mockImplementation(() => {})
  registerThemeSpy = spyOn(wasm, 'registerTheme').mockImplementation(() => {})
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
  // Test BUILD mode init (lines 68-73)
  it('should use default maps in non-watch mode on init', async () => {
    const asyncCallback = mock()
    const defaultClassMap = { test: 'classMap' }
    const defaultFileMap = { test: 'fileMap' }
    const defaultSheet = { test: 'sheet' }
    const theme = { colors: { primary: '#000' } }
    const t = {
      getOptions: () => ({
        package: 'package',
        cssDir: 'cssFile',
        watch: false,
        singleCss: true,
        theme,
        defaultClassMap,
        defaultFileMap,
        defaultSheet,
      }),
      async: mock().mockReturnValue(asyncCallback),
      resourcePath: 'nowatch-init.tsx',
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

    devupUILoader.bind(t as any)(Buffer.from('code'), 'nowatch-init.tsx')

    await waitFor(() => {
      expect(asyncCallback).toHaveBeenCalledWith(null, 'code', null)
    })

    // Verify non-watch init was executed (lines 68-73)
    expect(importFileMapSpy).toHaveBeenCalledWith(defaultFileMap)
    expect(importClassMapSpy).toHaveBeenCalledWith(defaultClassMap)
    expect(importSheetSpy).toHaveBeenCalledWith(defaultSheet)
    expect(registerThemeSpy).toHaveBeenCalledWith(theme)
  })

  // Test WATCH mode init (lines 55-67) + CSS writing (lines 94-111)
  it('should initialize watch mode and write css files', async () => {
    existsSyncSpy.mockReturnValue(true)
    readFileSyncSpy.mockReturnValue(
      '{"theme": {"colors": {"primary": "#fff"}}}',
    )
    exportSheetSpy.mockReturnValue('sheet')
    exportClassMapSpy.mockReturnValue('classMap')
    exportFileMapSpy.mockReturnValue('fileMap')
    getCssSpy.mockReturnValue('base-css')

    const asyncCallback = mock()
    const t = {
      getOptions: () => ({
        package: 'package',
        cssDir: 'cssDir',
        sheetFile: 'sheetFile',
        classMapFile: 'classMapFile',
        fileMapFile: 'fileMapFile',
        themeFile: 'themeFile',
        watch: true,
        singleCss: true,
      }),
      async: mock().mockReturnValue(asyncCallback),
      resourcePath: 'watch-init.tsx',
      addDependency: mock(),
    }

    codeExtractSpy.mockReturnValue({
      code: 'code',
      css: 'css',
      free: mock(),
      map: '{}',
      cssFile: 'devup-ui-1.css',
      updatedBaseStyle: true,
      [Symbol.dispose]: mock(),
    })

    devupUILoader.bind(t as any)(Buffer.from('code'), 'watch-init.tsx')

    await waitFor(() => {
      expect(asyncCallback).toHaveBeenCalledWith(null, 'code', {})
    })

    // Verify watch mode init was executed (lines 55-67)
    expect(existsSyncSpy).toHaveBeenCalledWith('sheetFile')
    expect(existsSyncSpy).toHaveBeenCalledWith('classMapFile')
    expect(existsSyncSpy).toHaveBeenCalledWith('fileMapFile')
    expect(existsSyncSpy).toHaveBeenCalledWith('themeFile')
    expect(registerThemeSpy).toHaveBeenCalledWith({
      colors: { primary: '#fff' },
    })

    // Verify updatedBaseStyle && watch branch (lines 94-99)
    expect(writeFileSpy).toHaveBeenCalledWith(
      join('cssDir', 'devup-ui.css'),
      'base-css',
      'utf-8',
    )

    // Verify cssFile && watch branch (lines 100-111)
    expect(writeFileSpy).toHaveBeenCalledWith(
      join('cssDir', 'devup-ui-1.css'),
      '/* watch-init.tsx 0 */',
    )
    expect(writeFileSpy).toHaveBeenCalledWith('sheetFile', 'sheet')
    expect(writeFileSpy).toHaveBeenCalledWith('classMapFile', 'classMap')
    expect(writeFileSpy).toHaveBeenCalledWith('fileMapFile', 'fileMap')
  })

  it('should extract code without css in watch mode', async () => {
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

    await waitFor(() => {
      expect(asyncCallback).toHaveBeenCalledWith(null, 'code', null)
    })
  })

  it('should extract code without css in build mode', async () => {
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
  })

  it('should handle error in build mode', async () => {
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

    await waitFor(() => {
      expect(asyncCallback).toHaveBeenCalledWith(new Error('error'))
    })
  })

  it('should handle error in watch mode', async () => {
    const asyncCallback = mock()
    const consoleErrorSpy = spyOn(console, 'error').mockImplementation(() => {})
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
      resourcePath: 'error-test.tsx',
      addDependency: mock(),
    }

    codeExtractSpy.mockImplementation(() => {
      throw new Error('extraction error')
    })

    devupUILoader.bind(t as any)(Buffer.from('code'), 'error-test.tsx')

    await waitFor(() => {
      expect(asyncCallback).toHaveBeenCalledWith(expect.any(Error))
    })
    consoleErrorSpy.mockRestore()
  })

  it('should use correct relative css path', async () => {
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

  it('should not write css files in build mode even with cssFile', async () => {
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
      css: 'css',
      free: mock(),
      map: '{}',
      cssFile: 'cssFile',
      updatedBaseStyle: true,
      [Symbol.dispose]: mock(),
    })
    devupUILoader.bind(t as any)(Buffer.from('code'), 'index.tsx')

    await waitFor(() => {
      expect(asyncCallback).toHaveBeenCalledWith(null, 'code', {})
    })
    // In build mode (watch=false), no CSS files should be written
    expect(writeFileSpy).not.toHaveBeenCalled()
  })

  describe('coordinator mode', () => {
    it('should delegate to coordinator via HTTP when coordinatorPortFile exists', async () => {
      existsSyncSpy.mockReturnValue(true)
      readFileSyncSpy.mockReturnValue('12345')

      const responseBody = JSON.stringify({
        code: 'coordinator code',
        map: '{"version":3}',
        cssFile: 'devup-ui-1.css',
        updatedBaseStyle: true,
      })

      const requestSpy = spyOn(http, 'request').mockImplementation(
        (_options: any, callback?: any) => {
          // Simulate a response
          const fakeRes = {
            statusCode: 200,
            on: mock((event: string, handler: (...args: unknown[]) => void) => {
              if (event === 'data') {
                handler(Buffer.from(responseBody))
              }
              if (event === 'end') {
                handler()
              }
              return fakeRes
            }),
          }
          if (callback) callback(fakeRes)
          return {
            on: mock(() => ({})),
            write: mock(),
            end: mock(),
          } as any
        },
      )

      const asyncCallback = mock()
      const t = {
        getOptions: () => ({
          package: 'package',
          cssDir: 'cssDir',
          sheetFile: 'sheetFile',
          classMapFile: 'classMapFile',
          fileMapFile: 'fileMapFile',
          themeFile: 'themeFile',
          watch: true,
          singleCss: true,
          coordinatorPortFile: 'coordinator.port',
        }),
        async: mock().mockReturnValue(asyncCallback),
        resourcePath: join(process.cwd(), 'src', 'App.tsx'),
        addDependency: mock(),
      }

      devupUILoader.bind(t as any)(Buffer.from('source code'), 'src/App.tsx')

      await waitFor(() => {
        expect(asyncCallback).toHaveBeenCalledWith(null, 'coordinator code', {
          version: 3,
        })
      })

      // Verify HTTP request was made
      expect(requestSpy).toHaveBeenCalledTimes(1)
      const reqOptions = requestSpy.mock.calls[0]![0] as Record<string, unknown>
      expect(reqOptions.hostname).toBe('127.0.0.1')
      expect(reqOptions.port).toBe(12345)
      expect(reqOptions.path).toBe('/extract')
      expect(reqOptions.method).toBe('POST')

      // Verify NO WASM functions were called
      expect(codeExtractSpy).not.toHaveBeenCalled()

      requestSpy.mockRestore()
    })

    it('should handle coordinator HTTP error', async () => {
      existsSyncSpy.mockReturnValue(true)
      readFileSyncSpy.mockReturnValue('12345')

      const requestSpy = spyOn(http, 'request').mockImplementation(
        (_options: any, _callback?: any) => {
          const fakeReq = {
            on: mock((event: string, handler: (...args: unknown[]) => void) => {
              if (event === 'error') {
                // Trigger error asynchronously
                setTimeout(() => handler(new Error('connection refused')), 0)
              }
              return fakeReq
            }),
            write: mock(),
            end: mock(),
          }
          return fakeReq as any
        },
      )

      const asyncCallback = mock()
      const t = {
        getOptions: () => ({
          package: 'package',
          cssDir: 'cssDir',
          sheetFile: 'sheetFile',
          classMapFile: 'classMapFile',
          fileMapFile: 'fileMapFile',
          themeFile: 'themeFile',
          watch: true,
          singleCss: true,
          coordinatorPortFile: 'coordinator.port',
        }),
        async: mock().mockReturnValue(asyncCallback),
        resourcePath: join(process.cwd(), 'src', 'App.tsx'),
        addDependency: mock(),
      }

      devupUILoader.bind(t as any)(Buffer.from('source code'), 'src/App.tsx')

      await waitFor(() => {
        expect(asyncCallback).toHaveBeenCalledWith(expect.any(Error))
      })

      requestSpy.mockRestore()
    })

    it('should handle coordinator non-200 response', async () => {
      existsSyncSpy.mockReturnValue(true)
      readFileSyncSpy.mockReturnValue('12345')

      const responseBody = JSON.stringify({
        error: 'extraction failed on server',
      })

      const requestSpy = spyOn(http, 'request').mockImplementation(
        (_options: any, callback?: any) => {
          const fakeRes = {
            statusCode: 500,
            on: mock((event: string, handler: (...args: unknown[]) => void) => {
              if (event === 'data') handler(Buffer.from(responseBody))
              if (event === 'end') handler()
              return fakeRes
            }),
          }
          if (callback) callback(fakeRes)
          return {
            on: mock(() => ({})),
            write: mock(),
            end: mock(),
          } as any
        },
      )

      const asyncCallback = mock()
      const t = {
        getOptions: () => ({
          package: 'package',
          cssDir: 'cssDir',
          sheetFile: 'sheetFile',
          classMapFile: 'classMapFile',
          fileMapFile: 'fileMapFile',
          themeFile: 'themeFile',
          watch: true,
          singleCss: true,
          coordinatorPortFile: 'coordinator.port',
        }),
        async: mock().mockReturnValue(asyncCallback),
        resourcePath: join(process.cwd(), 'src', 'App.tsx'),
        addDependency: mock(),
      }

      devupUILoader.bind(t as any)(Buffer.from('source code'), 'src/App.tsx')

      await waitFor(() => {
        expect(asyncCallback).toHaveBeenCalledWith(
          new Error('extraction failed on server'),
        )
      })

      requestSpy.mockRestore()
    })

    it('should handle malformed coordinator response', async () => {
      existsSyncSpy.mockReturnValue(true)
      readFileSyncSpy.mockReturnValue('12345')

      const requestSpy = spyOn(http, 'request').mockImplementation(
        (_options: any, callback?: any) => {
          const fakeRes = {
            statusCode: 200,
            on: mock((event: string, handler: (...args: unknown[]) => void) => {
              if (event === 'data') handler(Buffer.from('not json'))
              if (event === 'end') handler()
              return fakeRes
            }),
          }
          if (callback) callback(fakeRes)
          return {
            on: mock(() => ({})),
            write: mock(),
            end: mock(),
          } as any
        },
      )

      const asyncCallback = mock()
      const t = {
        getOptions: () => ({
          package: 'package',
          cssDir: 'cssDir',
          sheetFile: 'sheetFile',
          classMapFile: 'classMapFile',
          fileMapFile: 'fileMapFile',
          themeFile: 'themeFile',
          watch: true,
          singleCss: true,
          coordinatorPortFile: 'coordinator.port',
        }),
        async: mock().mockReturnValue(asyncCallback),
        resourcePath: join(process.cwd(), 'src', 'App.tsx'),
        addDependency: mock(),
      }

      devupUILoader.bind(t as any)(Buffer.from('source code'), 'src/App.tsx')

      await waitFor(() => {
        expect(asyncCallback).toHaveBeenCalledWith(expect.any(Error))
      })

      requestSpy.mockRestore()
    })

    it('should retry and error when coordinatorPortFile never appears', async () => {
      // existsSync always returns false — port file never appears
      existsSyncSpy.mockReturnValue(false)

      const asyncCallback = mock()
      const t = {
        getOptions: () => ({
          package: 'package',
          cssDir: 'cssDir',
          watch: true,
          singleCss: true,
          coordinatorPortFile: 'nonexistent.port',
          sheetFile: 'sheetFile',
          classMapFile: 'classMapFile',
          fileMapFile: 'fileMapFile',
          themeFile: 'themeFile',
        }),
        async: mock().mockReturnValue(asyncCallback),
        resourcePath: 'fallback.tsx',
        addDependency: mock(),
      }

      devupUILoader.bind(t as any)(Buffer.from('code'), 'fallback.tsx')

      // Retries 20 times × 50ms = 1s max, then calls back with error
      await waitFor(() => {
        expect(asyncCallback).toHaveBeenCalledWith(
          new Error('Coordinator port file not found'),
        )
      }, 3000)

      // WASM should NOT be used — coordinator mode does not fall back
      expect(codeExtractSpy).not.toHaveBeenCalled()
    })

    it('should retry and succeed when coordinatorPortFile appears after delay', async () => {
      // First few calls: port file doesn't exist, then it appears
      let callCount = 0
      existsSyncSpy.mockImplementation((path: string) => {
        if (path === 'coordinator.port') {
          callCount++
          return callCount > 3 // Appears on 4th check
        }
        return false
      })
      readFileSyncSpy.mockReturnValue('12345')

      const responseBody = JSON.stringify({
        code: 'coordinator code',
        map: undefined,
        cssFile: undefined,
        updatedBaseStyle: false,
      })

      const requestSpy = spyOn(http, 'request').mockImplementation(
        (_options: any, callback?: any) => {
          const fakeRes = {
            statusCode: 200,
            on: mock((event: string, handler: (...args: unknown[]) => void) => {
              if (event === 'data') handler(Buffer.from(responseBody))
              if (event === 'end') handler()
              return fakeRes
            }),
          }
          if (callback) callback(fakeRes)
          return {
            on: mock(() => ({})),
            write: mock(),
            end: mock(),
          } as any
        },
      )

      const asyncCallback = mock()
      const t = {
        getOptions: () => ({
          package: 'package',
          cssDir: 'cssDir',
          watch: true,
          singleCss: true,
          coordinatorPortFile: 'coordinator.port',
          sheetFile: 'sheetFile',
          classMapFile: 'classMapFile',
          fileMapFile: 'fileMapFile',
          themeFile: 'themeFile',
        }),
        async: mock().mockReturnValue(asyncCallback),
        resourcePath: join(process.cwd(), 'src', 'App.tsx'),
        addDependency: mock(),
      }

      devupUILoader.bind(t as any)(Buffer.from('code'), 'src/App.tsx')

      await waitFor(() => {
        expect(asyncCallback).toHaveBeenCalledWith(
          null,
          'coordinator code',
          null,
        )
      })

      expect(requestSpy).toHaveBeenCalledTimes(1)
      requestSpy.mockRestore()
    })

    it('should handle error when reading coordinator port file fails', async () => {
      existsSyncSpy.mockReturnValue(true)
      readFileSyncSpy.mockImplementation((path: string) => {
        if (path === 'coordinator.port') {
          throw new Error('EACCES: permission denied')
        }
        return '{}'
      })

      const asyncCallback = mock()
      const t = {
        getOptions: () => ({
          package: 'package',
          cssDir: 'cssDir',
          sheetFile: 'sheetFile',
          classMapFile: 'classMapFile',
          fileMapFile: 'fileMapFile',
          themeFile: 'themeFile',
          watch: true,
          singleCss: true,
          coordinatorPortFile: 'coordinator.port',
        }),
        async: mock().mockReturnValue(asyncCallback),
        resourcePath: join(process.cwd(), 'src', 'App.tsx'),
        addDependency: mock(),
      }

      devupUILoader.bind(t as any)(Buffer.from('source code'), 'src/App.tsx')

      await waitFor(() => {
        expect(asyncCallback).toHaveBeenCalledWith(
          new Error('EACCES: permission denied'),
        )
      })
    })

    it('should cache the coordinator port', async () => {
      existsSyncSpy.mockReturnValue(true)
      readFileSyncSpy.mockReturnValue('54321')

      const responseBody = JSON.stringify({
        code: 'code',
        map: undefined,
        cssFile: undefined,
        updatedBaseStyle: false,
      })

      const requestSpy = spyOn(http, 'request').mockImplementation(
        (_options: any, callback?: any) => {
          const fakeRes = {
            statusCode: 200,
            on: mock((event: string, handler: (...args: unknown[]) => void) => {
              if (event === 'data') handler(Buffer.from(responseBody))
              if (event === 'end') handler()
              return fakeRes
            }),
          }
          if (callback) callback(fakeRes)
          return {
            on: mock(() => ({})),
            write: mock(),
            end: mock(),
          } as any
        },
      )

      const makeContext = () => ({
        getOptions: () => ({
          package: 'package',
          cssDir: 'cssDir',
          sheetFile: 'sheetFile',
          classMapFile: 'classMapFile',
          fileMapFile: 'fileMapFile',
          themeFile: 'themeFile',
          watch: true,
          singleCss: true,
          coordinatorPortFile: 'coordinator.port',
        }),
        async: mock().mockReturnValue(mock()),
        resourcePath: join(process.cwd(), 'src', 'test.tsx'),
        addDependency: mock(),
      })

      // First call reads port from file
      devupUILoader.bind(makeContext() as any)(Buffer.from('code'), 'test.tsx')
      await waitFor(() => {
        expect(requestSpy).toHaveBeenCalledTimes(1)
      })

      // Second call should use cached port (readFileSync called only once for port)
      devupUILoader.bind(makeContext() as any)(Buffer.from('code'), 'test.tsx')
      await waitFor(() => {
        expect(requestSpy).toHaveBeenCalledTimes(2)
      })

      // readFileSync should only be called once for the port file
      // (existsSync is called each time, but readFileSync for port file only once due to caching)
      const portReads = readFileSyncSpy.mock.calls.filter(
        (call: unknown[]) => call[0] === 'coordinator.port',
      )
      expect(portReads.length).toBe(1)

      requestSpy.mockRestore()
    })
  })
})
