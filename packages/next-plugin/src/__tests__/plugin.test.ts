import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { join, resolve } from 'node:path'

import { getDefaultTheme, getThemeInterface, setPrefix } from '@devup-ui/wasm'
import { DevupUIWebpackPlugin } from '@devup-ui/webpack-plugin'

import { DevupUI } from '../plugin'
import { preload } from '../preload'

vi.mock('@devup-ui/webpack-plugin')
vi.mock('node:fs')
vi.mock('../preload')
vi.mock('@devup-ui/wasm', async (original) => ({
  ...(await original()),
  registerTheme: vi.fn(),
  getThemeInterface: vi.fn(),
  getDefaultTheme: vi.fn(),
  getCss: vi.fn(() => ''),
  setPrefix: vi.fn(),
  exportSheet: vi.fn(() =>
    JSON.stringify({
      css: {},
      font_faces: {},
      global_css_files: [],
      imports: {},
      keyframes: {},
      properties: {},
    }),
  ),
  exportClassMap: vi.fn(() => JSON.stringify({})),
  exportFileMap: vi.fn(() => JSON.stringify({})),
}))

describe('DevupUINextPlugin', () => {
  describe('webpack', () => {
    it('should apply webpack plugin', async () => {
      const ret = DevupUI({})

      ret.webpack!({ plugins: [] }, { buildId: 'tmpBuildId' } as any)

      expect(DevupUIWebpackPlugin).toHaveBeenCalledWith({
        cssDir: resolve('.next/cache', 'devup-ui_tmpBuildId'),
      })
    })

    it('should apply webpack plugin with dev', async () => {
      const ret = DevupUI({})

      ret.webpack!({ plugins: [] }, { buildId: 'tmpBuildId', dev: true } as any)

      expect(DevupUIWebpackPlugin).toHaveBeenCalledWith({
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

      expect(DevupUIWebpackPlugin).toHaveBeenCalledWith({
        package: 'new-package',
        cssDir: resolve('.next/cache', 'devup-ui_tmpBuildId'),
      })
    })

    it('should apply webpack plugin with webpack obj', async () => {
      const webpack = vi.fn()
      const ret = DevupUI(
        {
          webpack,
        },
        {
          package: 'new-package',
        },
      )

      ret.webpack!({ plugins: [] }, { buildId: 'tmpBuildId' } as any)

      expect(DevupUIWebpackPlugin).toHaveBeenCalledWith({
        package: 'new-package',
        cssDir: resolve('.next/cache', 'devup-ui_tmpBuildId'),
      })
      expect(webpack).toHaveBeenCalled()
    })
  })
  describe('turbo', () => {
    beforeEach(() => {
      // Mock fetch globally to prevent "http://localhost:undefined" errors
      global.fetch = vi.fn(() => Promise.resolve({} as Response))
    })

    afterEach(() => {
      vi.restoreAllMocks()
    })

    it('should apply turbo config', async () => {
      vi.stubEnv('TURBOPACK', '1')
      vi.mocked(existsSync)
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
      vi.stubEnv('TURBOPACK', '1')
      vi.mocked(existsSync).mockReturnValue(false)
      vi.mocked(mkdirSync).mockReturnValue('')
      vi.mocked(writeFileSync).mockReturnValue()
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
      expect(mkdirSync).toHaveBeenCalledWith('df', {
        recursive: true,
      })
      expect(writeFileSync).toHaveBeenCalledWith(join('df', '.gitignore'), '*')
    })
    it('should apply turbo config with exists df and devup.json', async () => {
      vi.stubEnv('TURBOPACK', '1')
      vi.mocked(existsSync).mockReturnValue(true)
      vi.mocked(readFileSync).mockReturnValue(
        JSON.stringify({ theme: 'theme' }),
      )
      vi.mocked(mkdirSync).mockReturnValue('')
      vi.mocked(writeFileSync).mockReturnValue()
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
      expect(mkdirSync).toHaveBeenCalledWith('df', {
        recursive: true,
      })
      expect(writeFileSync).toHaveBeenCalledWith(join('df', '.gitignore'), '*')
    })
    it('should throw error if NODE_ENV is production', () => {
      vi.stubEnv('NODE_ENV', 'production')
      vi.stubEnv('TURBOPACK', '1')
      vi.mocked(preload).mockReturnValue()
      const ret = DevupUI({})
      expect(ret).toEqual({
        turbopack: {
          rules: expect.any(Object),
        },
      })
      expect(preload).toHaveBeenCalledWith(
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
      vi.stubEnv('TURBOPACK', '1')
      vi.mocked(existsSync).mockReturnValue(true)
      vi.mocked(getThemeInterface).mockReturnValue('interface code')
      vi.mocked(readFileSync).mockReturnValue(
        JSON.stringify({ theme: 'theme' }),
      )
      vi.mocked(mkdirSync).mockReturnValue('')
      vi.mocked(writeFileSync).mockReturnValue()
      DevupUI({})
      expect(writeFileSync).toHaveBeenCalledWith(
        join('df', 'theme.d.ts'),
        'interface code',
      )
      expect(mkdirSync).toHaveBeenCalledWith('df', {
        recursive: true,
      })
      expect(writeFileSync).toHaveBeenCalledWith(join('df', '.gitignore'), '*')
    })
    it('should set DEVUP_UI_DEFAULT_THEME when getDefaultTheme returns a value', async () => {
      vi.stubEnv('TURBOPACK', '1')
      vi.stubEnv('DEVUP_UI_DEFAULT_THEME', '')
      vi.mocked(existsSync)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(false)
      vi.mocked(getDefaultTheme).mockReturnValue('dark')
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
      vi.stubEnv('TURBOPACK', '1')
      vi.stubEnv('DEVUP_UI_DEFAULT_THEME', '')
      vi.mocked(existsSync)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(false)
      vi.mocked(getDefaultTheme).mockReturnValue(undefined)
      const config: any = {}
      const ret = DevupUI(config)

      expect(process.env.DEVUP_UI_DEFAULT_THEME).toBe('')
      expect(ret.env).toBeUndefined()
      expect(config.env).toBeUndefined()
    })
    it('should set DEVUP_UI_DEFAULT_THEME and preserve existing env vars', async () => {
      vi.stubEnv('TURBOPACK', '1')
      vi.stubEnv('DEVUP_UI_DEFAULT_THEME', '')
      vi.mocked(existsSync)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(false)
      vi.mocked(getDefaultTheme).mockReturnValue('light')
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
      vi.stubEnv('TURBOPACK', '1')
      vi.mocked(existsSync)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(false)
      DevupUI({}, { prefix: 'my-prefix' })
      expect(setPrefix).toHaveBeenCalledWith('my-prefix')
    })
    it('should handle debugPort fetch failure in development mode', async () => {
      vi.stubEnv('TURBOPACK', '1')
      vi.stubEnv('NODE_ENV', 'development')
      vi.stubEnv('PORT', '3000')
      vi.mocked(existsSync)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(false)
      vi.mocked(writeFileSync).mockReturnValue()

      // Mock process.exit to prevent actual exit
      const originalExit = process.exit
      const exitSpy = vi.fn()
      process.exit = exitSpy as any

      // Mock process.debugPort
      const originalDebugPort = process.debugPort
      process.debugPort = 9229

      // Mock fetch globally before calling DevupUI
      const originalFetch = global.fetch
      const fetchMock = vi.fn((url: string | URL) => {
        const urlString = typeof url === 'string' ? url : url.toString()
        if (urlString.includes('9229')) {
          return Promise.reject(new Error('Connection refused'))
        }
        return Promise.resolve({} as Response)
      })
      global.fetch = fetchMock as any

      // Use fake timers to control setTimeout
      vi.useFakeTimers()

      try {
        DevupUI({})

        // Wait for the fetch promise to reject (this triggers the catch handler)
        // The catch handler sets up a setTimeout, so we need to wait for that
        await vi.runAllTimersAsync()

        // Verify process.exit was called with code 77
        expect(exitSpy).toHaveBeenCalledWith(77)
      } finally {
        // Restore
        vi.useRealTimers()
        global.fetch = originalFetch
        process.exit = originalExit
        process.debugPort = originalDebugPort
      }
    })
  })
})
