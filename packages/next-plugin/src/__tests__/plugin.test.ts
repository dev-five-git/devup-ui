import { join, resolve } from 'node:path'

import { afterEach, beforeEach, describe, expect, it, mock } from 'bun:test'

const mockExistsSync = mock((_path: string) => false)
const mockMkdirSync = mock((_path: string, _options?: object) => '')
const mockReadFileSync = mock(
  (_path: string, _encoding?: string) => '{}' as string,
)
const mockWriteFileSync = mock(
  (_path: string, _data: string, _options?: string) => {},
)

mock.module('node:fs', () => ({
  existsSync: mockExistsSync,
  mkdirSync: mockMkdirSync,
  readFileSync: mockReadFileSync,
  writeFileSync: mockWriteFileSync,
}))

const mockGetDefaultTheme = mock(() => undefined as string | undefined)
const mockGetThemeInterface = mock(() => '')
const mockSetPrefix = mock((_prefix: string) => {})
const mockRegisterTheme = mock()
const mockGetCss = mock(() => '')
const mockExportSheet = mock(() =>
  JSON.stringify({
    css: {},
    font_faces: {},
    global_css_files: [],
    imports: {},
    keyframes: {},
    properties: {},
  }),
)
const mockExportClassMap = mock(() => JSON.stringify({}))
const mockExportFileMap = mock(() => JSON.stringify({}))

mock.module('@devup-ui/wasm', () => ({
  registerTheme: mockRegisterTheme,
  getThemeInterface: mockGetThemeInterface,
  getDefaultTheme: mockGetDefaultTheme,
  getCss: mockGetCss,
  setPrefix: mockSetPrefix,
  exportSheet: mockExportSheet,
  exportClassMap: mockExportClassMap,
  exportFileMap: mockExportFileMap,
}))

const mockDevupUIWebpackPlugin = mock()

mock.module('@devup-ui/webpack-plugin', () => ({
  DevupUIWebpackPlugin: mockDevupUIWebpackPlugin,
}))

const mockPreload = mock()

mock.module('../preload', () => ({
  preload: mockPreload,
}))

import { DevupUI } from '../plugin'

let originalEnv: NodeJS.ProcessEnv
let originalFetch: typeof global.fetch
let originalDebugPort: number

beforeEach(() => {
  mockExistsSync.mockReset()
  mockMkdirSync.mockReset()
  mockReadFileSync.mockReset()
  mockWriteFileSync.mockReset()
  mockGetDefaultTheme.mockReset()
  mockGetThemeInterface.mockReset()
  mockSetPrefix.mockReset()
  mockRegisterTheme.mockReset()
  mockGetCss.mockReset()
  mockExportSheet.mockReset()
  mockExportClassMap.mockReset()
  mockExportFileMap.mockReset()
  mockDevupUIWebpackPlugin.mockReset()
  mockPreload.mockReset()

  mockExistsSync.mockReturnValue(false)
  mockReadFileSync.mockReturnValue('{}')
  mockGetDefaultTheme.mockReturnValue(undefined)
  mockGetThemeInterface.mockReturnValue('')
  mockGetCss.mockReturnValue('')
  mockExportSheet.mockReturnValue(
    JSON.stringify({
      css: {},
      font_faces: {},
      global_css_files: [],
      imports: {},
      keyframes: {},
      properties: {},
    }),
  )
  mockExportClassMap.mockReturnValue(JSON.stringify({}))
  mockExportFileMap.mockReturnValue(JSON.stringify({}))

  originalEnv = { ...process.env }
  originalFetch = global.fetch
  originalDebugPort = process.debugPort
  global.fetch = mock(() => Promise.resolve({} as Response)) as any
})

afterEach(() => {
  process.env = originalEnv
  global.fetch = originalFetch
  process.debugPort = originalDebugPort
})

describe('DevupUINextPlugin', () => {
  describe('webpack', () => {
    it('should apply webpack plugin', async () => {
      const ret = DevupUI({})

      ret.webpack!({ plugins: [] }, { buildId: 'tmpBuildId' } as any)

      expect(mockDevupUIWebpackPlugin).toHaveBeenCalledWith({
        cssDir: resolve('.next/cache', 'devup-ui_tmpBuildId'),
      })
    })

    it('should apply webpack plugin with dev', async () => {
      const ret = DevupUI({})

      ret.webpack!({ plugins: [] }, { buildId: 'tmpBuildId', dev: true } as any)

      expect(mockDevupUIWebpackPlugin).toHaveBeenCalledWith({
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

      expect(mockDevupUIWebpackPlugin).toHaveBeenCalledWith({
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

      expect(mockDevupUIWebpackPlugin).toHaveBeenCalledWith({
        package: 'new-package',
        cssDir: resolve('.next/cache', 'devup-ui_tmpBuildId'),
      })
      expect(webpack).toHaveBeenCalled()
    })
  })
  describe('turbo', () => {
    it('should apply turbo config', async () => {
      process.env.TURBOPACK = '1'
      mockExistsSync
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
      mockExistsSync.mockReturnValue(false)
      mockMkdirSync.mockReturnValue('')
      mockWriteFileSync.mockReturnValue(undefined)
      const ret = DevupUI({})

      expect(ret).toEqual({
        turbopack: {
          rules: {
            './df/devup-ui/*.css': [
              {
                loader: '@devup-ui/next-plugin/css-loader',
                options: {
                  watch: false,
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
      expect(mockMkdirSync).toHaveBeenCalledWith('df', {
        recursive: true,
      })
      expect(mockWriteFileSync).toHaveBeenCalledWith(
        join('df', '.gitignore'),
        '*',
      )
    })
    it('should apply turbo config with exists df and devup.json', async () => {
      process.env.TURBOPACK = '1'
      mockExistsSync.mockReturnValue(true)
      mockReadFileSync.mockReturnValue(JSON.stringify({ theme: 'theme' }))
      mockMkdirSync.mockReturnValue('')
      mockWriteFileSync.mockReturnValue(undefined)
      const ret = DevupUI({})

      expect(ret).toEqual({
        turbopack: {
          rules: {
            './df/devup-ui/*.css': [
              {
                loader: '@devup-ui/next-plugin/css-loader',
                options: {
                  watch: false,
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
      expect(mockMkdirSync).not.toHaveBeenCalled()
      // gitignore is also NOT written when it exists
      expect(mockWriteFileSync).not.toHaveBeenCalledWith(
        join('df', '.gitignore'),
        '*',
      )
    })
    it('should throw error if NODE_ENV is production', () => {
      ;(process.env as any).NODE_ENV = 'production'
      process.env.TURBOPACK = '1'
      mockPreload.mockReturnValue(undefined)
      const ret = DevupUI({})
      expect(ret).toEqual({
        turbopack: {
          rules: expect.any(Object),
        },
      })
      expect(mockPreload).toHaveBeenCalledWith(
        new RegExp(
          `(node_modules(?!.*(${['@devup-ui']
            .join('|')
            .replaceAll('/', '[\\/\\\\_]')})([\\/\\\\.]|$)))|(.mdx.[tj]sx?$)`,
        ),
        '@devup-ui/react',
        false,
        expect.any(String),
        [],
      )
    })
    it('should create theme.d.ts file', async () => {
      process.env.TURBOPACK = '1'
      mockExistsSync.mockReturnValue(true)
      mockGetThemeInterface.mockReturnValue('interface code')
      mockReadFileSync.mockReturnValue(JSON.stringify({ theme: 'theme' }))
      mockMkdirSync.mockReturnValue('')
      mockWriteFileSync.mockReturnValue(undefined)
      DevupUI({})
      expect(mockWriteFileSync).toHaveBeenCalledWith(
        join('df', 'theme.d.ts'),
        'interface code',
      )
      // mkdirSync is NOT called when existsSync returns true
      expect(mockMkdirSync).not.toHaveBeenCalled()
    })
    it('should set DEVUP_UI_DEFAULT_THEME when getDefaultTheme returns a value', async () => {
      process.env.TURBOPACK = '1'
      process.env.DEVUP_UI_DEFAULT_THEME = ''
      mockExistsSync
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(false)
      mockGetDefaultTheme.mockReturnValue('dark')
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
      mockExistsSync
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(false)
      mockGetDefaultTheme.mockReturnValue(undefined)
      const config: any = {}
      const ret = DevupUI(config)

      expect(process.env.DEVUP_UI_DEFAULT_THEME).toBe('')
      expect(ret.env).toBeUndefined()
      expect(config.env).toBeUndefined()
    })
    it('should set DEVUP_UI_DEFAULT_THEME and preserve existing env vars', async () => {
      process.env.TURBOPACK = '1'
      process.env.DEVUP_UI_DEFAULT_THEME = ''
      mockExistsSync
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(false)
      mockGetDefaultTheme.mockReturnValue('light')
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
      mockExistsSync
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(false)
      DevupUI({}, { prefix: 'my-prefix' })
      expect(mockSetPrefix).toHaveBeenCalledWith('my-prefix')
    })
    it('should handle debugPort fetch failure in development mode', async () => {
      process.env.TURBOPACK = '1'
      ;(process.env as any).NODE_ENV = 'development'
      process.env.PORT = '3000'
      mockExistsSync
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(false)
      mockWriteFileSync.mockReturnValue(undefined)

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
