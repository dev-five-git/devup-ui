import { existsSync, mkdirSync, writeFileSync } from 'node:fs'
import { join, resolve } from 'node:path'

import { DevupUIWebpackPlugin } from '@devup-ui/webpack-plugin'

import { DevupUI } from '../plugin'

vi.mock('@devup-ui/webpack-plugin')
vi.mock('node:fs')

describe('plugin', () => {
  describe('webpack', () => {
    it('should apply webpack plugin', async () => {
      const ret = DevupUI({})

      ret.webpack!({ plugins: [] }, {} as any)

      expect(DevupUIWebpackPlugin).toHaveBeenCalledWith({})
    })

    it('should apply webpack plugin with config', async () => {
      const ret = DevupUI(
        {},
        {
          package: 'new-package',
        },
      )

      ret.webpack!({ plugins: [] }, {} as any)

      expect(DevupUIWebpackPlugin).toHaveBeenCalledWith({
        package: 'new-package',
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

      ret.webpack!({ plugins: [] }, {} as any)

      expect(DevupUIWebpackPlugin).toHaveBeenCalledWith({
        package: 'new-package',
      })
      expect(webpack).toHaveBeenCalled()
    })
  })
  describe('turbo', () => {
    it('should apply turbo config', async () => {
      vi.stubEnv('TURBOPACK', '1')
      vi.mocked(existsSync).mockReturnValue(true)
      const ret = DevupUI({})

      expect(ret.experimental).toEqual({
        turbo: {
          rules: {
            'devup-ui.css': [
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
                  cssFile: resolve('.df', 'devup-ui.css'),
                  sheetFile: join('.df', 'sheet.json'),
                  classMapFile: join('.df', 'classMap.json'),
                  watch: false,
                },
              },
            ],
          },
        },
      })
    })
    it('should apply turbo config with create .df', async () => {
      vi.stubEnv('TURBOPACK', '1')
      vi.mocked(existsSync).mockReturnValue(false)
      vi.mocked(mkdirSync).mockReturnValue('')
      vi.mocked(writeFileSync).mockReturnValue()
      const ret = DevupUI({})

      expect(ret.experimental).toEqual({
        turbo: {
          rules: {
            'devup-ui.css': [
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
                  cssFile: resolve('.df', 'devup-ui.css'),
                  sheetFile: join('.df', 'sheet.json'),
                  classMapFile: join('.df', 'classMap.json'),
                  watch: false,
                },
              },
            ],
          },
        },
      })
      expect(mkdirSync).toHaveBeenCalledWith('.df')
      expect(writeFileSync).toHaveBeenCalledWith(
        resolve('.df', 'devup-ui.css'),
        '/* devup-ui */',
      )
    })
  })
})
