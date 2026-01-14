import * as fs from 'node:fs'
import { join, resolve } from 'node:path'

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

import { DevupUI } from '../plugin'
import * as preloadModule from '../preload'

let existsSyncSpy: ReturnType<typeof spyOn>
let mkdirSyncSpy: ReturnType<typeof spyOn>
let readFileSyncSpy: ReturnType<typeof spyOn>
let writeFileSyncSpy: ReturnType<typeof spyOn>
let getDefaultThemeSpy: ReturnType<typeof spyOn>
let getThemeInterfaceSpy: ReturnType<typeof spyOn>
let setPrefixSpy: ReturnType<typeof spyOn>
let registerThemeSpy: ReturnType<typeof spyOn>
let getCssSpy: ReturnType<typeof spyOn>
let exportSheetSpy: ReturnType<typeof spyOn>
let exportClassMapSpy: ReturnType<typeof spyOn>
let exportFileMapSpy: ReturnType<typeof spyOn>
let devupUIWebpackPluginSpy: ReturnType<typeof spyOn>
let preloadSpy: ReturnType<typeof spyOn>

let originalEnv: NodeJS.ProcessEnv
let originalFetch: typeof global.fetch
let originalDebugPort: number

beforeEach(() => {
  existsSyncSpy = spyOn(fs, 'existsSync').mockReturnValue(false)
  mkdirSyncSpy = spyOn(fs, 'mkdirSync').mockReturnValue('' as any)
  readFileSyncSpy = spyOn(fs, 'readFileSync').mockReturnValue('{}')
  writeFileSyncSpy = spyOn(fs, 'writeFileSync').mockReturnValue(undefined)
  getDefaultThemeSpy = spyOn(wasm, 'getDefaultTheme').mockReturnValue(undefined)
  getThemeInterfaceSpy = spyOn(wasm, 'getThemeInterface').mockReturnValue('')
  setPrefixSpy = spyOn(wasm, 'setPrefix').mockReturnValue(undefined)
  registerThemeSpy = spyOn(wasm, 'registerTheme').mockReturnValue(undefined)
  getCssSpy = spyOn(wasm, 'getCss').mockReturnValue('')
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
  devupUIWebpackPluginSpy = spyOn(
    webpackPluginModule,
    'DevupUIWebpackPlugin',
  ).mockImplementation(mock() as never)
  preloadSpy = spyOn(preloadModule, 'preload').mockReturnValue(undefined)

  originalEnv = { ...process.env }
  originalFetch = global.fetch
  originalDebugPort = process.debugPort
  global.fetch = mock(() => Promise.resolve({} as Response)) as any
})

afterEach(() => {
  process.env = originalEnv
  global.fetch = originalFetch
  process.debugPort = originalDebugPort
  existsSyncSpy.mockRestore()
  mkdirSyncSpy.mockRestore()
  readFileSyncSpy.mockRestore()
  writeFileSyncSpy.mockRestore()
  getDefaultThemeSpy.mockRestore()
  getThemeInterfaceSpy.mockRestore()
  setPrefixSpy.mockRestore()
  registerThemeSpy.mockRestore()
  getCssSpy.mockRestore()
  exportSheetSpy.mockRestore()
  exportClassMapSpy.mockRestore()
  exportFileMapSpy.mockRestore()
  devupUIWebpackPluginSpy.mockRestore()
  preloadSpy.mockRestore()
})

describe('DevupUINextPlugin', () => {
  describe('webpack', () => {
    it('should apply webpack plugin', async () => {
      const ret = DevupUI({})

      ret.webpack!({ plugins: [] }, { buildId: 'tmpBuildId' } as any)

      expect(devupUIWebpackPluginSpy).toHaveBeenCalledWith({
        cssDir: resolve('.next/cache', 'devup-ui_tmpBuildId'),
      })
    })

    it('should apply webpack plugin with dev', async () => {
      const ret = DevupUI({})

      ret.webpack!({ plugins: [] }, { buildId: 'tmpBuildId', dev: true } as any)

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

      ret.webpack!({ plugins: [] }, { buildId: 'tmpBuildId' } as any)

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

      ret.webpack!({ plugins: [] }, { buildId: 'tmpBuildId' } as any)

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
            '*.{tsx,ts,js,mjs}': {
              loaders: [
                {
                  loader: '@devup-ui/next-plugin/loader',
                  options: {
                    package: '@devup-ui/react',
                    cssDir: resolve('df', 'devup-ui'),
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
                    `(node_modules(?!.*(${['@devup-ui']
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
            '*.{tsx,ts,js,mjs}': {
              condition: {
                not: {
                  path: new RegExp(
                    `(node_modules(?!.*(${['@devup-ui']
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
            '*.{tsx,ts,js,mjs}': {
              condition: {
                not: {
                  path: new RegExp(
                    `(node_modules(?!.*(${['@devup-ui']
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
    it('should throw error if NODE_ENV is production', () => {
      ;(process.env as any).NODE_ENV = 'production'
      process.env.TURBOPACK = '1'
      preloadSpy.mockReturnValue(undefined)
      const ret = DevupUI({})
      expect(ret).toEqual({
        turbopack: {
          rules: expect.any(Object),
        },
      })
      expect(preloadSpy).toHaveBeenCalledWith(
        new RegExp(
          `(node_modules(?!.*(${['@devup-ui']
            .join('|')
            .replaceAll('/', '[\\/\\\\_]')})([\\/\\\\.]|$)))|(.mdx.[tj]sx?$)`,
        ),
        '@devup-ui/react',
        false,
        expect.any(String),
        [],
        {
          '@emotion/styled': 'styled',
          '@vanilla-extract/css': null,
          'styled-components': 'styled',
        },
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
    it('should handle debugPort fetch failure in development mode', async () => {
      process.env.TURBOPACK = '1'
      ;(process.env as any).NODE_ENV = 'development'
      process.env.PORT = '3000'
      existsSyncSpy
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(false)
      writeFileSyncSpy.mockReturnValue(undefined)

      // Mock process.exit to prevent actual exit
      const originalExit = process.exit
      const exitSpy = mock()
      process.exit = exitSpy as any

      // Mock process.debugPort
      process.debugPort = 9229

      // Mock fetch globally before calling DevupUI
      const fetchMock = mock((url: string | URL) => {
        const urlString = typeof url === 'string' ? url : url.toString()
        if (urlString.includes('9229')) {
          return Promise.reject(new Error('Connection refused'))
        }
        return Promise.resolve({} as Response)
      })
      global.fetch = fetchMock as any

      try {
        DevupUI({})

        // Wait for the fetch promise to reject and setTimeout to fire (500ms in plugin.ts + buffer)
        await new Promise((resolve) => setTimeout(resolve, 600))

        // Verify process.exit was called with code 77
        expect(exitSpy).toHaveBeenCalledWith(77)
      } finally {
        // Restore
        process.exit = originalExit
      }
    })
  })
})
