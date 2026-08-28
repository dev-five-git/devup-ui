import * as fs from 'node:fs'
import { join, resolve } from 'node:path'

import type { StaticImportGraph } from '@devup-ui/plugin-utils'
import * as importGraphModule from '@devup-ui/plugin-utils'
import * as wasm from '@devup-ui/wasm'
import * as webpackPluginModule from '@devup-ui/webpack-plugin'
import {
  afterEach,
  beforeEach,
  describe,
  expect,
  it,
  mock,
  spyOn,
} from 'bun:test'

import * as coordinatorModule from '../coordinator'
import { DevupUI, selectWasmVariant } from '../plugin'
import { setWasmForTesting } from '../wasm'

type CodeExtractResult = ReturnType<typeof wasm.codeExtract>
type NextWebpackConfig = Parameters<
  NonNullable<ReturnType<typeof DevupUI>['webpack']>
>[0]
type NextWebpackContext = Parameters<
  NonNullable<ReturnType<typeof DevupUI>['webpack']>
>[1]

function createWebpackConfig(): NextWebpackConfig {
  return { plugins: [] } as unknown as NextWebpackConfig
}

function createWebpackContext(
  overrides: Partial<NextWebpackContext> = {},
): NextWebpackContext {
  return { buildId: 'tmpBuildId', ...overrides } as NextWebpackContext
}

function setNodeEnv(value: string): void {
  process.env.NODE_ENV = value
}

function createCodeExtractResult(contents: string): CodeExtractResult {
  return {
    css: '',
    code: contents,
    cssFile: '',
    map: undefined,
    updatedBaseStyle: false,
    free: mock(),
    [Symbol.dispose]: mock(),
  } as unknown as CodeExtractResult
}

let existsSyncSpy: ReturnType<typeof spyOn>
let mkdirSyncSpy: ReturnType<typeof spyOn>
let readFileSyncSpy: ReturnType<typeof spyOn>
let writeFileSyncSpy: ReturnType<typeof spyOn>
let unlinkSyncSpy: ReturnType<typeof spyOn>
let getDefaultThemeSpy: ReturnType<typeof spyOn>
let getThemeInterfaceSpy: ReturnType<typeof spyOn>
let setPrefixSpy: ReturnType<typeof spyOn>
let registerThemeSpy: ReturnType<typeof spyOn>
let getCssSpy: ReturnType<typeof spyOn>
let importSheetSpy: ReturnType<typeof spyOn>
let importClassMapSpy: ReturnType<typeof spyOn>
let importFileMapSpy: ReturnType<typeof spyOn>
let exportSheetSpy: ReturnType<typeof spyOn>
let exportClassMapSpy: ReturnType<typeof spyOn>
let exportFileMapSpy: ReturnType<typeof spyOn>
let codeExtractSpy: ReturnType<typeof spyOn>
let codeExtractWithoutSourceMapSpy: ReturnType<typeof spyOn>
let devupUIWebpackPluginSpy: ReturnType<typeof spyOn>
let startCoordinatorSpy: ReturnType<typeof spyOn>

let originalEnv: NodeJS.ProcessEnv
let originalFetch: typeof global.fetch
let originalDebugPort: number

beforeEach(() => {
  existsSyncSpy = spyOn(fs, 'existsSync').mockReturnValue(false)
  mkdirSyncSpy = spyOn(fs, 'mkdirSync').mockReturnValue(undefined)
  readFileSyncSpy = spyOn(fs, 'readFileSync').mockReturnValue('{}')
  writeFileSyncSpy = spyOn(fs, 'writeFileSync').mockReturnValue(undefined)
  unlinkSyncSpy = spyOn(fs, 'unlinkSync').mockReturnValue(undefined)
  getDefaultThemeSpy = spyOn(wasm, 'getDefaultTheme').mockReturnValue(undefined)
  getThemeInterfaceSpy = spyOn(wasm, 'getThemeInterface').mockReturnValue('')
  setPrefixSpy = spyOn(wasm, 'setPrefix').mockReturnValue(undefined)
  registerThemeSpy = spyOn(wasm, 'registerTheme').mockReturnValue(undefined)
  getCssSpy = spyOn(wasm, 'getCss').mockReturnValue('')
  importSheetSpy = spyOn(wasm, 'importSheet').mockReturnValue(undefined)
  importClassMapSpy = spyOn(wasm, 'importClassMap').mockReturnValue(undefined)
  importFileMapSpy = spyOn(wasm, 'importFileMap').mockReturnValue(undefined)
  exportSheetSpy = spyOn(wasm, 'exportSheet').mockReturnValue(
    JSON.stringify({
      css: {},
      font_faces: {},
      global_css_files: [],
      imports: {},
      keyframes: {},
      properties: {},
    }),
  )
  exportClassMapSpy = spyOn(wasm, 'exportClassMap').mockReturnValue(
    JSON.stringify({}),
  )
  exportFileMapSpy = spyOn(wasm, 'exportFileMap').mockReturnValue(
    JSON.stringify({}),
  )
  codeExtractSpy = spyOn(wasm, 'codeExtract').mockImplementation(
    (_path: string, contents: string) => createCodeExtractResult(contents),
  )
  codeExtractWithoutSourceMapSpy = spyOn(
    wasm,
    'codeExtractWithoutSourceMap',
  ).mockImplementation((_path: string, contents: string) =>
    createCodeExtractResult(contents),
  )
  devupUIWebpackPluginSpy = spyOn(
    webpackPluginModule,
    'DevupUIWebpackPlugin',
  ).mockImplementation(mock() as never)
  startCoordinatorSpy = spyOn(
    coordinatorModule,
    'startCoordinator',
  ).mockReturnValue({ close: mock() as () => void })
  setWasmForTesting(wasm)

  originalEnv = { ...process.env }
  originalFetch = global.fetch
  originalDebugPort = process.debugPort
  global.fetch = mock(() => Promise.resolve({} as Response))
})

afterEach(() => {
  setWasmForTesting(undefined)
  process.env = originalEnv
  global.fetch = originalFetch
  process.debugPort = originalDebugPort
  existsSyncSpy.mockRestore()
  mkdirSyncSpy.mockRestore()
  readFileSyncSpy.mockRestore()
  writeFileSyncSpy.mockRestore()
  unlinkSyncSpy.mockRestore()
  getDefaultThemeSpy.mockRestore()
  getThemeInterfaceSpy.mockRestore()
  setPrefixSpy.mockRestore()
  registerThemeSpy.mockRestore()
  getCssSpy.mockRestore()
  importSheetSpy.mockRestore()
  importClassMapSpy.mockRestore()
  importFileMapSpy.mockRestore()
  exportSheetSpy.mockRestore()
  exportClassMapSpy.mockRestore()
  exportFileMapSpy.mockRestore()
  codeExtractSpy.mockRestore()
  codeExtractWithoutSourceMapSpy.mockRestore()
  devupUIWebpackPluginSpy.mockRestore()
  startCoordinatorSpy.mockRestore()
})

describe('DevupUINextPlugin', () => {
  it('selects the lite engine only when the graph has no vanilla-extract file', () => {
    expect(selectWasmVariant(undefined)).toBe('full')
    expect(
      selectWasmVariant({ files: ['src/page.tsx'] } as StaticImportGraph),
    ).toBe('lite')
    expect(
      selectWasmVariant({ files: ['src/theme.css.ts'] } as StaticImportGraph),
    ).toBe('full')
    expect(
      selectWasmVariant({ files: ['src/theme.css.js'] } as StaticImportGraph),
    ).toBe('full')
    expect(
      selectWasmVariant({ files: ['src/page.tsx'] } as StaticImportGraph, [
        'node_modules/design-system/theme.css.ts',
      ]),
    ).toBe('full')
  })

  describe('webpack', () => {
    it('should apply webpack plugin', async () => {
      const ret = DevupUI({})

      ret.webpack!(createWebpackConfig(), createWebpackContext())

      expect(devupUIWebpackPluginSpy).toHaveBeenCalledWith({
        cssDir: resolve('.next/cache', 'devup-ui_tmpBuildId'),
      })
    })

    it('should apply webpack plugin with dev', async () => {
      const ret = DevupUI({})

      ret.webpack!(createWebpackConfig(), createWebpackContext({ dev: true }))

      expect(devupUIWebpackPluginSpy).toHaveBeenCalledWith({
        cssDir: resolve('df', 'devup-ui_tmpBuildId'),
        watch: true,
      })
    })

    it('should apply webpack plugin with config', async () => {
      const ret = DevupUI(
        {},
        {
          package: 'new-package',
        },
      )

      ret.webpack!(createWebpackConfig(), createWebpackContext())

      expect(devupUIWebpackPluginSpy).toHaveBeenCalledWith({
        package: 'new-package',
        cssDir: resolve('.next/cache', 'devup-ui_tmpBuildId'),
      })
    })

    it('should apply webpack plugin with webpack obj', async () => {
      const webpack = mock()
      const ret = DevupUI(
        {
          webpack,
        },
        {
          package: 'new-package',
        },
      )

      ret.webpack!(createWebpackConfig(), createWebpackContext())

      expect(devupUIWebpackPluginSpy).toHaveBeenCalledWith({
        package: 'new-package',
        cssDir: resolve('.next/cache', 'devup-ui_tmpBuildId'),
      })
      expect(webpack).toHaveBeenCalled()
    })
  })
  describe('turbo', () => {
    it('should apply turbo config', async () => {
      process.env.TURBOPACK = '1'
      existsSyncSpy
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(false)
      const ret = DevupUI({})

      expect(ret).toEqual({
        turbopack: {
          rules: {
            './df/devup-ui/*.css': [
              {
                loader: '@devup-ui/next-plugin/css-loader',
                options: {
                  watch: false,
                  coordinatorPortFile: join('df', 'coordinator.port'),
                  sheetFile: join('df', 'sheet.json'),
                  classMapFile: join('df', 'classMap.json'),
                  fileMapFile: join('df', 'fileMap.json'),
                  themeFile: 'devup.json',
                  theme: {},
                  defaultClassMap: {},
                  defaultFileMap: {},
                  defaultSheet: {
                    css: {},
                    font_faces: {},
                    global_css_files: [],
                    imports: {},
                    keyframes: {},
                    properties: {},
                  },
                },
              },
            ],
            '*.{tsx,ts,jsx,js,mjs}': {
              loaders: [
                {
                  loader: '@devup-ui/next-plugin/loader',
                  options: {
                    package: '@devup-ui/react',
                    cssDir: resolve('df', 'devup-ui'),
                    coordinatorPortFile: join('df', 'coordinator.port'),
                    sheetFile: join('df', 'sheet.json'),
                    classMapFile: join('df', 'classMap.json'),
                    fileMapFile: join('df', 'fileMap.json'),
                    themeFile: 'devup.json',
                    watch: false,
                    singleCss: false,
                    theme: {},
                    defaultClassMap: {},
                    defaultFileMap: {},
                    importAliases: {
                      '@emotion/styled': 'styled',
                      '@vanilla-extract/css': null,
                      'styled-components': 'styled',
                    },
                    defaultSheet: {
                      css: {},
                      font_faces: {},
                      global_css_files: [],
                      imports: {},
                      keyframes: {},
                      properties: {},
                    },
                  },
                },
              ],
              condition: {
                not: {
                  path: new RegExp(
                    `(node_modules(?!.*(${['@devup-ui', '@devup-editor']
                      .join('|')
                      .replaceAll(
                        '/',
                        '[\\/\\\\_]',
                      )})([\\/\\\\.]|$)))|(.mdx.[tj]sx?$)`,
                  ),
                },
              },
            },
          },
        },
      })
    })
    it('should apply turbo config with create df', async () => {
      process.env.TURBOPACK = '1'
      existsSyncSpy.mockReturnValue(false)
      mkdirSyncSpy.mockReturnValue('')
      writeFileSyncSpy.mockReturnValue(undefined)
      const ret = DevupUI({})

      expect(ret).toEqual({
        turbopack: {
          rules: {
            './df/devup-ui/*.css': [
              {
                loader: '@devup-ui/next-plugin/css-loader',
                options: {
                  watch: false,
                  coordinatorPortFile: join('df', 'coordinator.port'),
                  sheetFile: join('df', 'sheet.json'),
                  classMapFile: join('df', 'classMap.json'),
                  fileMapFile: join('df', 'fileMap.json'),
                  themeFile: 'devup.json',
                  theme: {},
                  defaultClassMap: {},
                  defaultFileMap: {},
                  defaultSheet: {
                    css: {},
                    font_faces: {},
                    global_css_files: [],
                    imports: {},
                    keyframes: {},
                    properties: {},
                  },
                },
              },
            ],
            '*.{tsx,ts,jsx,js,mjs}': {
              condition: {
                not: {
                  path: new RegExp(
                    `(node_modules(?!.*(${['@devup-ui', '@devup-editor']
                      .join('|')
                      .replaceAll(
                        '/',
                        '[\\/\\\\_]',
                      )})([\\/\\\\.]|$)))|(.mdx.[tj]sx?$)`,
                  ),
                },
              },
              loaders: [
                {
                  loader: '@devup-ui/next-plugin/loader',
                  options: {
                    package: '@devup-ui/react',
                    cssDir: resolve('df', 'devup-ui'),
                    coordinatorPortFile: join('df', 'coordinator.port'),
                    sheetFile: join('df', 'sheet.json'),
                    classMapFile: join('df', 'classMap.json'),
                    fileMapFile: join('df', 'fileMap.json'),
                    importAliases: {
                      '@emotion/styled': 'styled',
                      '@vanilla-extract/css': null,
                      'styled-components': 'styled',
                    },
                    watch: false,
                    singleCss: false,
                    theme: {},
                    defaultClassMap: {},
                    defaultFileMap: {},
                    defaultSheet: {
                      css: {},
                      font_faces: {},
                      global_css_files: [],
                      imports: {},
                      keyframes: {},
                      properties: {},
                    },
                    themeFile: 'devup.json',
                  },
                },
              ],
            },
          },
        },
      })
      expect(mkdirSyncSpy).toHaveBeenCalledWith('df', {
        recursive: true,
      })
      expect(writeFileSyncSpy).toHaveBeenCalledWith(
        join('df', '.gitignore'),
        '*',
      )
    })
    it('should apply turbo config with exists df and devup.json', async () => {
      process.env.TURBOPACK = '1'
      existsSyncSpy.mockReturnValue(true)
      readFileSyncSpy.mockReturnValue(JSON.stringify({ theme: 'theme' }))
      mkdirSyncSpy.mockReturnValue('')
      writeFileSyncSpy.mockReturnValue(undefined)
      const ret = DevupUI({})

      expect(ret).toEqual({
        turbopack: {
          rules: {
            './df/devup-ui/*.css': [
              {
                loader: '@devup-ui/next-plugin/css-loader',
                options: {
                  watch: false,
                  coordinatorPortFile: join('df', 'coordinator.port'),
                  sheetFile: join('df', 'sheet.json'),
                  classMapFile: join('df', 'classMap.json'),
                  fileMapFile: join('df', 'fileMap.json'),
                  themeFile: 'devup.json',
                  theme: 'theme',
                  defaultClassMap: {},
                  defaultFileMap: {},
                  defaultSheet: {
                    css: {},
                    font_faces: {},
                    global_css_files: [],
                    imports: {},
                    keyframes: {},
                    properties: {},
                  },
                },
              },
            ],
            '*.{tsx,ts,jsx,js,mjs}': {
              condition: {
                not: {
                  path: new RegExp(
                    `(node_modules(?!.*(${['@devup-ui', '@devup-editor']
                      .join('|')
                      .replaceAll(
                        '/',
                        '[\\/\\\\_]',
                      )})([\\/\\\\.]|$)))|(.mdx.[tj]sx?$)`,
                  ),
                },
              },
              loaders: [
                {
                  loader: '@devup-ui/next-plugin/loader',
                  options: {
                    package: '@devup-ui/react',
                    cssDir: resolve('df', 'devup-ui'),
                    coordinatorPortFile: join('df', 'coordinator.port'),
                    sheetFile: join('df', 'sheet.json'),
                    classMapFile: join('df', 'classMap.json'),
                    fileMapFile: join('df', 'fileMap.json'),
                    watch: false,
                    singleCss: false,
                    theme: 'theme',
                    defaultClassMap: {},
                    defaultFileMap: {},
                    importAliases: {
                      '@emotion/styled': 'styled',
                      '@vanilla-extract/css': null,
                      'styled-components': 'styled',
                    },
                    defaultSheet: {
                      css: {},
                      font_faces: {},
                      global_css_files: [],
                      imports: {},
                      keyframes: {},
                      properties: {},
                    },
                    themeFile: 'devup.json',
                  },
                },
              ],
            },
          },
        },
      })
      // mkdirSync is NOT called when existsSync returns true
      expect(mkdirSyncSpy).not.toHaveBeenCalled()
      // gitignore is also NOT written when it exists
      expect(writeFileSyncSpy).not.toHaveBeenCalledWith(
        join('df', '.gitignore'),
        '*',
      )
    })
    it('should start coordinator even in production mode', () => {
      setNodeEnv('production')
      process.env.TURBOPACK = '1'
      existsSyncSpy
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(false)
      const ret = DevupUI({})
      expect(ret).toEqual({
        turbopack: {
          rules: expect.any(Object),
        },
      })
      expect(startCoordinatorSpy).toHaveBeenCalledWith({
        wasm,
        package: '@devup-ui/react',
        cssDir: resolve('df', 'devup-ui'),
        singleCss: false,
        sheetFile: join('df', 'sheet.json'),
        classMapFile: join('df', 'classMap.json'),
        fileMapFile: join('df', 'fileMap.json'),
        importAliases: {
          '@emotion/styled': 'styled',
          '@vanilla-extract/css': null,
          'styled-components': 'styled',
        },
        coordinatorPortFile: join('df', 'coordinator.port'),
        canonicalMap: expect.any(Object),
        expectedBaseFiles: expect.any(Array),
        prewarmedFiles: expect.any(Array),
        prewarmedOutputs: expect.any(Map),
        sourceMap: false,
      })
    })
    it('keeps source maps when Next production browser source maps are enabled', () => {
      setNodeEnv('production')
      process.env.TURBOPACK = '1'

      DevupUI({ productionBrowserSourceMaps: true })

      expect(startCoordinatorSpy).toHaveBeenCalledWith(
        expect.objectContaining({ sourceMap: true }),
      )
    })
    it('should create theme.d.ts file', async () => {
      process.env.TURBOPACK = '1'
      existsSyncSpy.mockReturnValue(true)
      getThemeInterfaceSpy.mockReturnValue('interface code')
      readFileSyncSpy.mockReturnValue(JSON.stringify({ theme: 'theme' }))
      mkdirSyncSpy.mockReturnValue('')
      writeFileSyncSpy.mockReturnValue(undefined)
      DevupUI({})
      expect(writeFileSyncSpy).toHaveBeenCalledWith(
        join('df', 'theme.d.ts'),
        'interface code',
      )
      // mkdirSync is NOT called when existsSync returns true
      expect(mkdirSyncSpy).not.toHaveBeenCalled()
    })
    it('should set DEVUP_UI_DEFAULT_THEME when getDefaultTheme returns a value', async () => {
      process.env.TURBOPACK = '1'
      process.env.DEVUP_UI_DEFAULT_THEME = ''
      existsSyncSpy
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(false)
      getDefaultThemeSpy.mockReturnValue('dark')
      const config: any = {}
      const ret = DevupUI(config)

      expect(process.env.DEVUP_UI_DEFAULT_THEME).toBe('dark')
      expect(ret.env).toEqual({
        DEVUP_UI_DEFAULT_THEME: 'dark',
      })
      expect(config.env).toEqual({
        DEVUP_UI_DEFAULT_THEME: 'dark',
      })
    })
    it('should not set DEVUP_UI_DEFAULT_THEME when getDefaultTheme returns undefined', async () => {
      process.env.TURBOPACK = '1'
      process.env.DEVUP_UI_DEFAULT_THEME = ''
      existsSyncSpy
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(false)
      getDefaultThemeSpy.mockReturnValue(undefined)
      const config: any = {}
      const ret = DevupUI(config)

      expect(process.env.DEVUP_UI_DEFAULT_THEME).toBe('')
      expect(ret.env).toBeUndefined()
      expect(config.env).toBeUndefined()
    })
    it('should set DEVUP_UI_DEFAULT_THEME and preserve existing env vars', async () => {
      process.env.TURBOPACK = '1'
      process.env.DEVUP_UI_DEFAULT_THEME = ''
      existsSyncSpy
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(false)
      getDefaultThemeSpy.mockReturnValue('light')
      const config: any = {
        env: {
          CUSTOM_VAR: 'value',
        },
      }
      const ret = DevupUI(config)

      expect(process.env.DEVUP_UI_DEFAULT_THEME).toBe('light')
      expect(ret.env).toEqual({
        CUSTOM_VAR: 'value',
        DEVUP_UI_DEFAULT_THEME: 'light',
      })
      expect(config.env).toEqual({
        CUSTOM_VAR: 'value',
        DEVUP_UI_DEFAULT_THEME: 'light',
      })
    })
    it('should call setPrefix when prefix option is provided', async () => {
      process.env.TURBOPACK = '1'
      existsSyncSpy
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(false)
      DevupUI({}, { prefix: 'my-prefix' })
      expect(setPrefixSpy).toHaveBeenCalledWith('my-prefix')
    })
    it('should import previous session state on restart', () => {
      process.env.TURBOPACK = '1'
      existsSyncSpy
        .mockReturnValueOnce(true) // distDir
        .mockReturnValueOnce(true) // cssDir
        .mockReturnValueOnce(true) // gitignoreFile
        .mockReturnValueOnce(false) // devupFile in loadDevupConfigSync

      // Simulate previous session state files on disk
      const prevSheet = {
        css: { a: 'color:red' },
        font_faces: {},
        global_css_files: [],
        imports: {},
        keyframes: {},
        properties: {},
      }
      const prevClassMap = { a: 0 }
      const prevFileMap = { 'src/App.tsx': 0 }

      readFileSyncSpy
        .mockReturnValueOnce(JSON.stringify(prevSheet)) // sheetFile
        .mockReturnValueOnce(JSON.stringify(prevClassMap)) // classMapFile
        .mockReturnValueOnce(JSON.stringify(prevFileMap)) // fileMapFile

      DevupUI({})

      // Verify previous state was imported before registerTheme
      expect(importSheetSpy).toHaveBeenCalledWith(prevSheet)
      expect(importClassMapSpy).toHaveBeenCalledWith(prevClassMap)
      expect(importFileMapSpy).toHaveBeenCalledWith(prevFileMap)
      expect(registerThemeSpy).toHaveBeenCalledWith({})

      // Verify stale port file was deleted before starting coordinator
      expect(unlinkSyncSpy).toHaveBeenCalledWith(join('df', 'coordinator.port'))
    })
    it('should handle missing state files gracefully on first run', () => {
      process.env.TURBOPACK = '1'
      existsSyncSpy
        .mockReturnValueOnce(false) // distDir — doesn't exist
        .mockReturnValueOnce(false) // cssDir
        .mockReturnValueOnce(false) // gitignoreFile
        .mockReturnValueOnce(false) // devupFile

      // readFileSync throws for state files (they don't exist)
      readFileSyncSpy.mockImplementation((path: string) => {
        throw new Error(`ENOENT: no such file or directory, open '${path}'`)
      })

      // Should not throw — try-catch handles missing files
      DevupUI({})

      // importSheet should NOT have been called (readFileSync threw)
      expect(importSheetSpy).not.toHaveBeenCalled()
      expect(importClassMapSpy).not.toHaveBeenCalled()
      expect(importFileMapSpy).not.toHaveBeenCalled()

      // registerTheme should still be called with empty theme
      expect(registerThemeSpy).toHaveBeenCalledWith({})
    })
    it('should start coordinator in development mode', async () => {
      process.env.TURBOPACK = '1'
      setNodeEnv('development')
      existsSyncSpy
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(false)
      writeFileSyncSpy.mockReturnValue(undefined)

      const closeMock = mock() as () => void
      startCoordinatorSpy.mockReturnValue({ close: closeMock })

      const exitHandlers: (() => void)[] = []
      const processOnSpy = spyOn(process, 'on').mockImplementation(
        (event: string, handler: (...args: unknown[]) => void) => {
          if (event === 'exit') exitHandlers.push(handler as () => void)
          return process
        },
      )

      DevupUI({})

      // Verify coordinator was started with correct options
      expect(startCoordinatorSpy).toHaveBeenCalledWith({
        wasm,
        package: '@devup-ui/react',
        cssDir: resolve('df', 'devup-ui'),
        singleCss: false,
        sheetFile: join('df', 'sheet.json'),
        classMapFile: join('df', 'classMap.json'),
        fileMapFile: join('df', 'fileMap.json'),
        importAliases: {
          '@emotion/styled': 'styled',
          '@vanilla-extract/css': null,
          'styled-components': 'styled',
        },
        coordinatorPortFile: join('df', 'coordinator.port'),
        canonicalMap: expect.any(Object),
        expectedBaseFiles: expect.any(Array),
        prewarmedFiles: [],
        prewarmedOutputs: expect.any(Map),
        sourceMap: true,
      })
      expect(codeExtractSpy).not.toHaveBeenCalled()
      expect(codeExtractWithoutSourceMapSpy).not.toHaveBeenCalled()

      // Verify initial CSS file is written
      expect(writeFileSyncSpy).toHaveBeenCalledWith(
        join(resolve('df', 'devup-ui'), 'devup-ui.css'),
        '',
      )

      // Verify exit handler was registered and calls coordinator.close()
      expect(exitHandlers.length).toBeGreaterThan(0)
      exitHandlers[0]!()
      expect(closeMock).toHaveBeenCalledTimes(1)

      // Verify --inspect-brk env vars are NOT set
      expect(process.env.TURBOPACK_DEBUG_JS).toBeUndefined()
      expect(process.env.NODE_OPTIONS ?? '').not.toContain('--inspect-brk')

      processOnSpy.mockRestore()
    })

    it('hands the coordinator the full compiled-file set, not the static-only route map', () => {
      process.env.TURBOPACK = '1'
      process.env.DEVUP_UI_PROFILE = '1'
      const profileSpy = spyOn(console, 'info').mockImplementation(() => {})
      // The base sheet must wait for lazily-loaded modules too, so
      // expectedBaseFiles comes from computeCompiledFiles (static + dynamic
      // edges) rather than computeFileRoutes (static edges only). Using the
      // route map here served the sheet before any dynamic() module extracted.
      const compiledSpy = spyOn(
        importGraphModule,
        'computeCompiledFiles',
      ).mockReturnValue(['src/app/page.tsx', 'src/lazy/panel.tsx'])
      const routesSpy = spyOn(
        importGraphModule,
        'computeFileRoutes',
      ).mockReturnValue({ 'src/app/page.tsx': [0] })
      const events: string[] = []
      codeExtractWithoutSourceMapSpy.mockImplementation(
        (filename: string, contents: string) => {
          events.push(`extract:${filename}`)
          return createCodeExtractResult(contents)
        },
      )
      startCoordinatorSpy.mockImplementation(() => {
        events.push('startCoordinator')
        return { close: mock() as () => void }
      })
      try {
        DevupUI({})

        expect(startCoordinatorSpy).toHaveBeenCalledWith(
          expect.objectContaining({
            expectedBaseFiles: ['src/app/page.tsx', 'src/lazy/panel.tsx'],
            prewarmedFiles: ['src/app/page.tsx', 'src/lazy/panel.tsx'],
          }),
        )
        expect(codeExtractWithoutSourceMapSpy).toHaveBeenCalledTimes(2)
        expect(codeExtractWithoutSourceMapSpy).toHaveBeenCalledWith(
          'src/app/page.tsx',
          '{}',
          '@devup-ui/react',
          expect.any(String),
          false,
          false,
          true,
          expect.anything(),
        )
        expect(codeExtractWithoutSourceMapSpy).toHaveBeenCalledWith(
          'src/lazy/panel.tsx',
          '{}',
          '@devup-ui/react',
          expect.any(String),
          false,
          false,
          true,
          expect.anything(),
        )
        expect(events).toEqual([
          'extract:src/app/page.tsx',
          'extract:src/lazy/panel.tsx',
          'startCoordinator',
        ])
        const profiles: Record<string, unknown>[] = profileSpy.mock.calls
          .map(([value]) => value)
          .filter(
            (value): value is string =>
              typeof value === 'string' &&
              value.startsWith('[devup-ui:profile] '),
          )
          .map((value) => JSON.parse(value.slice('[devup-ui:profile] '.length)))
        const prewarmProfile = profiles.find(
          ({ phase }) => phase === 'next.prewarm',
        )
        expect(prewarmProfile).toMatchObject({
          collectMs: expect.any(Number),
          extractMs: expect.any(Number),
          files: 2,
          phase: 'next.prewarm',
          readMs: expect.any(Number),
          sourceBytes: Buffer.byteLength('{}') * 2,
        })
        expect(profiles).toEqual(
          expect.arrayContaining([
            expect.objectContaining({ phase: 'next.initialCss' }),
            expect.objectContaining({ phase: 'next.stateSnapshot' }),
            expect.objectContaining({ phase: 'next.setup' }),
          ]),
        )
        // the static-only route map is not consulted outside atom-hoist mode
        expect(routesSpy).not.toHaveBeenCalled()
      } finally {
        profileSpy.mockRestore()
        compiledSpy.mockRestore()
        routesSpy.mockRestore()
      }
    })

    it('prewarms source candidates hidden from the route closure', () => {
      process.env.TURBOPACK = '1'
      const page = resolve('src/app/page.tsx')
      const templateTarget = resolve('src/demos/template-target.tsx')
      const graphSpy = spyOn(
        importGraphModule,
        'buildStaticImportGraph',
      ).mockReturnValue({
        files: [page, templateTarget],
        fileSet: new Set([page, templateTarget]),
        staticImports: new Map([
          [page, new Set<string>()],
          [templateTarget, new Set<string>()],
        ]),
        staticImporters: new Map([
          [page, new Set<string>()],
          [templateTarget, new Set<string>()],
        ]),
        dynamicTargets: new Set(),
        dynamicImports: new Map([
          [page, new Set<string>()],
          [templateTarget, new Set<string>()],
        ]),
        externalImports: new Map([
          [page, new Set<string>()],
          [templateTarget, new Set<string>()],
        ]),
      })
      const compiledSpy = spyOn(
        importGraphModule,
        'computeCompiledFiles',
      ).mockReturnValue(['src/app/page.tsx'])
      try {
        DevupUI({})

        expect(startCoordinatorSpy).toHaveBeenCalledWith(
          expect.objectContaining({
            expectedBaseFiles: ['src/app/page.tsx'],
            prewarmedFiles: [
              'src/app/page.tsx',
              'src/demos/template-target.tsx',
            ],
          }),
        )
        expect(codeExtractWithoutSourceMapSpy).toHaveBeenCalledTimes(2)
      } finally {
        graphSpy.mockRestore()
        compiledSpy.mockRestore()
      }
    })

    it('prewarms the same complete file set in singleCss mode', () => {
      process.env.TURBOPACK = '1'
      const compiledSpy = spyOn(
        importGraphModule,
        'computeCompiledFiles',
      ).mockReturnValue(['src/app/page.tsx', 'src/app/card.tsx'])
      try {
        DevupUI({}, { singleCss: true })

        expect(codeExtractWithoutSourceMapSpy).toHaveBeenCalledTimes(2)
        expect(codeExtractWithoutSourceMapSpy).toHaveBeenCalledWith(
          'src/app/card.tsx',
          '{}',
          '@devup-ui/react',
          expect.any(String),
          true,
          false,
          true,
          expect.anything(),
        )
        expect(startCoordinatorSpy).toHaveBeenCalledWith(
          expect.objectContaining({
            singleCss: true,
            prewarmedFiles: ['src/app/card.tsx', 'src/app/page.tsx'],
          }),
        )
      } finally {
        compiledSpy.mockRestore()
      }
    })

    it('does not enable atom hoisting when atomHoist option is unset', () => {
      process.env.TURBOPACK = '1'
      const setAtomHoistSpy = spyOn(wasm, 'setAtomHoist').mockReturnValue(
        undefined,
      )
      const importFileRoutesSpy = spyOn(
        wasm,
        'importFileRoutes',
      ).mockReturnValue(undefined)
      const importCanonicalMapSpy = spyOn(
        wasm,
        'importCanonicalMap',
      ).mockReturnValue(undefined)
      try {
        DevupUI({})
        expect(setAtomHoistSpy).not.toHaveBeenCalled()
        expect(importFileRoutesSpy).not.toHaveBeenCalled()
        // single-importer collapse still runs (it is the always-on default)
        expect(importCanonicalMapSpy).toHaveBeenCalled()
      } finally {
        setAtomHoistSpy.mockRestore()
        importFileRoutesSpy.mockRestore()
        importCanonicalMapSpy.mockRestore()
      }
    })

    it('composes atom hoisting WITH single-importer collapse when atomHoist is set', () => {
      process.env.TURBOPACK = '1'
      // 4 distinct leaf routes (ids 0..3); layout shared by all four.
      const computeSpy = spyOn(
        importGraphModule,
        'computeFileRoutes',
      ).mockReturnValue({
        'src/app/layout.tsx': [0, 1, 2, 3],
        'src/app/a/page.tsx': [0],
        'src/app/b/page.tsx': [1],
        'src/app/c/page.tsx': [2],
        'src/app/d/page.tsx': [3],
      })
      const importFileRoutesSpy = spyOn(
        wasm,
        'importFileRoutes',
      ).mockReturnValue(undefined)
      const setAtomHoistSpy = spyOn(wasm, 'setAtomHoist').mockReturnValue(
        undefined,
      )
      // Collapse and atom hoisting COMPOSE: importCanonicalMap must STILL run.
      const importCanonicalMapSpy = spyOn(
        wasm,
        'importCanonicalMap',
      ).mockReturnValue(undefined)
      try {
        DevupUI({}, { atomHoist: 2 })
        // reach folded by bucket; mocked FS => empty canonical map => bucket==file
        expect(importFileRoutesSpy).toHaveBeenCalledWith({
          'src/app/layout.tsx': [0, 1, 2, 3],
          'src/app/a/page.tsx': [0],
          'src/app/b/page.tsx': [1],
          'src/app/c/page.tsx': [2],
          'src/app/d/page.tsx': [3],
        })
        // direct threshold, clamped to >= 2
        expect(setAtomHoistSpy).toHaveBeenCalledWith(2)
        // composition: collapse pre-pass also ran
        expect(importCanonicalMapSpy).toHaveBeenCalled()
      } finally {
        computeSpy.mockRestore()
        importFileRoutesSpy.mockRestore()
        setAtomHoistSpy.mockRestore()
        importCanonicalMapSpy.mockRestore()
      }
    })

    it('clamps the atomHoist threshold to a minimum of 2', () => {
      process.env.TURBOPACK = '1'
      const computeSpy = spyOn(
        importGraphModule,
        'computeFileRoutes',
      ).mockReturnValue({
        'src/app/a/page.tsx': [0],
        'src/app/b/page.tsx': [1],
      })
      const setAtomHoistSpy = spyOn(wasm, 'setAtomHoist').mockReturnValue(
        undefined,
      )
      try {
        DevupUI({}, { atomHoist: 1 })
        // max(2, 1) === 2
        expect(setAtomHoistSpy).toHaveBeenCalledWith(2)
      } finally {
        computeSpy.mockRestore()
        setAtomHoistSpy.mockRestore()
      }
    })

    it('keeps atom hoisting off when fewer than two routes exist', () => {
      process.env.TURBOPACK = '1'
      const computeSpy = spyOn(
        importGraphModule,
        'computeFileRoutes',
      ).mockReturnValue({ 'src/app/a/page.tsx': [0] })
      const setAtomHoistSpy = spyOn(wasm, 'setAtomHoist').mockReturnValue(
        undefined,
      )
      try {
        DevupUI({}, { atomHoist: 2 })
        expect(setAtomHoistSpy).not.toHaveBeenCalled()
      } finally {
        computeSpy.mockRestore()
        setAtomHoistSpy.mockRestore()
      }
    })
  })
})
