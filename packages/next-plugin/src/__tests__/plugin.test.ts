import { existsSync, mkdirSync, writeFileSync } from 'node:fs'
import { join, resolve } from 'node:path'

import { DevupUIWebpackPlugin } from '@devup-ui/webpack-plugin'

import { DevupUI } from '../plugin'

vi.mock('@devup-ui/webpack-plugin')
vi.mock('node:fs')

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
      vi.mocked(existsSync).mockReturnValue(true)
      const ret = DevupUI({})

      expect(ret).toEqual({
        turbopack: {
          rules: {
            'devup-ui': [
              {
                loader: '@devup-ui/webpack-plugin/css-loader',
                options: {
                  watch: false,
                },
              },
            ],
            '*.{tsx,ts,js,mjs}': [
              {
                loader: '@devup-ui/webpack-plugin/loader',
                options: {
                  package: '@devup-ui/react',
                  cssDir: resolve('df', 'devup-ui'),
                  sheetFile: join('df', 'sheet.json'),
                  classMapFile: join('df', 'classMap.json'),
                  fileMapFile: join('df', 'fileMap.json'),
                  watch: false,
                  singleCss: false,
                },
              },
            ],
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
            'devup-ui': [
              {
                loader: '@devup-ui/webpack-plugin/css-loader',
                options: {
                  watch: false,
                },
              },
            ],
            '*.{tsx,ts,js,mjs}': [
              {
                loader: '@devup-ui/webpack-plugin/loader',
                options: {
                  package: '@devup-ui/react',
                  cssDir: resolve('df', 'devup-ui'),
                  sheetFile: join('df', 'sheet.json'),
                  classMapFile: join('df', 'classMap.json'),
                  fileMapFile: join('df', 'fileMap.json'),
                  watch: false,
                  singleCss: false,
                },
              },
            ],
          },
        },
      })
      expect(mkdirSync).toHaveBeenCalledWith('df')
      // expect(writeFileSync).toHaveBeenCalledWith(
      //   resolve('df', 'devup-ui', 'devup-ui.css'),
      //   '/* devup-ui */',
      // )
      expect(writeFileSync).toHaveBeenCalledWith(join('df', '.gitignore'), '*')
    })
  })
})
