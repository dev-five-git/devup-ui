import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { join, resolve } from 'node:path'

import { getThemeInterface } from '@devup-ui/wasm'
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
  })
})
