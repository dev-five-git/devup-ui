import {
  existsSync,
  mkdirSync,
  readFileSync,
  stat,
  writeFileSync,
} from 'node:fs'
import { join, resolve } from 'node:path'

import {
  getCss,
  getDefaultTheme,
  getThemeInterface,
  registerTheme,
} from '@devup-ui/wasm'
import { describe } from 'vitest'

import { DevupUIWebpackPlugin } from '../plugin'

vi.mock('@devup-ui/wasm')
vi.mock('node:fs')

beforeEach(() => {
  vi.resetAllMocks()
})
afterAll(() => {
  vi.restoreAllMocks()
})

describe('devupUIPlugin', () => {
  console.error = vi.fn()
  describe('no watch', () => {
    it('should apply default options', () => {
      import.meta.resolve = vi.fn().mockReturnValue('resolved')
      expect(new DevupUIWebpackPlugin({}).options).toEqual({
        include: [],
        package: '@devup-ui/react',
        cssFile: resolve('df', 'devup-ui.css'),
        devupPath: 'devup.json',
        interfacePath: 'df',
        watch: false,
        debug: false,
      })
    })

    it('should apply custom options', () => {
      import.meta.resolve = vi.fn().mockReturnValue('resolved')
      expect(
        new DevupUIWebpackPlugin({
          package: 'new-package',
          cssFile: 'new-css-file',
          devupPath: 'new-devup-path',
          interfacePath: 'new-interface-path',
          watch: false,
        }).options,
      ).toEqual({
        include: [],
        package: 'new-package',
        cssFile: 'new-css-file',
        devupPath: 'new-devup-path',
        interfacePath: 'new-interface-path',
        watch: false,
        debug: false,
      })
    })

    it('should write data files', () => {
      vi.mocked(readFileSync).mockReturnValue('{"theme": "theme"}')
      vi.mocked(getThemeInterface).mockReturnValue('interfaceCode')
      vi.mocked(getCss).mockReturnValue('css')
      vi.mocked(existsSync).mockReturnValue(false)
      vi.mocked(writeFileSync).mockReturnValue()
      vi.mocked(mkdirSync)

      const plugin = new DevupUIWebpackPlugin({})
      plugin.writeDataFiles()

      expect(readFileSync).toHaveBeenCalledWith('devup.json', 'utf-8')
      expect(registerTheme).toHaveBeenCalledWith('theme')
      expect(getThemeInterface).toHaveBeenCalledWith(
        '@devup-ui/react',
        'DevupThemeColors',
        'DevupThemeTypography',
        'DevupTheme',
      )
      expect(writeFileSync).toHaveBeenCalledWith(
        join('df', 'theme.d.ts'),
        'interfaceCode',
        {
          encoding: 'utf-8',
        },
      )
    })

    it('should catch error', () => {
      vi.mocked(existsSync).mockReturnValue(true)
      vi.mocked(stat).mockImplementation((_, callback) => {
        ;(callback as any)(new Error('error'), null)
      })
      vi.mocked(readFileSync).mockImplementation(() => {
        throw new Error('error')
      })
      const plugin = new DevupUIWebpackPlugin({
        devupPath: 'custom-devup.json',
      })

      vi.mocked(getCss).mockReturnValue('css')
      const compiler = {
        options: {
          module: {
            rules: [],
          },
          plugins: [],
        },
        hooks: {
          afterCompile: {
            tap: vi.fn(),
          },
          done: {
            tap: vi.fn(),
          },
          watchRun: {
            tapAsync: vi.fn(),
          },
        },
        webpack: {
          DefinePlugin: vi.fn(),
        },
      } as any
      plugin.apply(compiler)
      // asyncCompile
      const add = vi.fn()
      vi.mocked(compiler.hooks.afterCompile.tap).mock.calls[0][1]({
        fileDependencies: {
          add,
        },
      })
      expect(add).toHaveBeenCalledWith(resolve('custom-devup.json'))
      expect(compiler.hooks.done.tap).toHaveBeenCalled()

      vi.mocked(compiler.hooks.done.tap).mock.calls[0][1]({
        hasErrors: () => true,
      })
      // expect(writeFileSync).not.toHaveBeenCalled()

      vi.mocked(compiler.hooks.done.tap).mock.calls[0][1]({
        hasErrors: () => false,
      })
      expect(writeFileSync).toHaveBeenCalledWith(
        resolve('df', 'devup-ui.css'),
        'css',
        {
          encoding: 'utf-8',
        },
      )
    })

    it('should skip writing css file', () => {
      vi.mocked(readFileSync).mockReturnValue('{"theme": "theme"}')
      vi.mocked(getThemeInterface).mockReturnValue('interfaceCode')
      vi.mocked(getCss).mockReturnValue('css')
      vi.mocked(existsSync).mockReturnValue(false)
      vi.mocked(writeFileSync).mockReturnValue()
      vi.mocked(mkdirSync)

      const plugin = new DevupUIWebpackPlugin({
        cssFile: 'css',
      })
      plugin.apply({
        options: {
          module: {
            rules: [],
          },
          plugins: [],
        },
        hooks: {
          afterCompile: {
            tap: vi.fn(),
          },
          done: {
            tap: vi.fn(),
          },
          watchRun: {
            tapAsync: vi.fn(),
          },
        },
        webpack: {
          DefinePlugin: vi.fn(),
        },
      } as any)

      expect(writeFileSync).toHaveBeenCalledWith('css', '', {
        encoding: 'utf-8',
      })
    })
  })
  describe('watch', () => {
    it('should write css file', () => {
      vi.mocked(readFileSync).mockReturnValue('{"theme": "theme"}')
      vi.mocked(getThemeInterface).mockReturnValue('interfaceCode')
      vi.mocked(getCss).mockReturnValue('css')
      vi.mocked(writeFileSync).mockReturnValue()
      vi.mocked(mkdirSync)

      const plugin = new DevupUIWebpackPlugin({
        watch: true,
      })
      const compiler = {
        options: {
          module: {
            rules: [],
          },
          plugins: [],
        },
        hooks: {
          afterCompile: {
            tap: vi.fn(),
          },
          done: {
            tap: vi.fn(),
          },
          watchRun: {
            tapAsync: vi.fn(),
          },
        },
        webpack: {
          DefinePlugin: vi.fn(),
        },
      } as any
      plugin.apply(compiler)

      expect(writeFileSync).toHaveBeenCalledWith(
        resolve('df', 'devup-ui.css'),
        '',
        {
          encoding: 'utf-8',
        },
      )
      expect(compiler.hooks.done.tap).not.toHaveBeenCalled()
    })
    it('should register devup watch', () => {
      const plugin = new DevupUIWebpackPlugin({
        watch: true,
      })
      const compiler = {
        options: {
          module: {
            rules: [],
          },
          plugins: [],
        },
        hooks: {
          afterCompile: {
            tap: vi.fn(),
          },
          done: {
            tap: vi.fn(),
          },
          watchRun: {
            tapAsync: vi.fn(),
          },
        },
        webpack: {
          DefinePlugin: vi.fn(),
        },
      } as any
      vi.mocked(existsSync).mockReturnValue(true)
      plugin.apply(compiler)
      // watchRun
      const callback = vi.fn()
      vi.mocked(compiler.hooks.watchRun.tapAsync).mock.calls[0][1](
        null,
        callback,
      )
      expect(callback).toHaveBeenCalled()
      expect(registerTheme).toBeCalledTimes(0)

      vi.mocked(stat).mockImplementation((_, callback) => {
        ;(callback as any)(null, { mtimeMs: 1 })
      })
      vi.mocked(readFileSync).mockReturnValue('{"theme": "theme"}')
      vi.mocked(writeFileSync).mockReturnValue()
      vi.mocked(registerTheme).mockReturnValue()
      vi.mocked(stat).mockImplementation((_, callback) => {
        ;(callback as any)(null, { mtimeMs: 2 })
      })
      vi.mocked(console.error).mockReturnValue()

      plugin.apply(compiler as any)

      vi.mocked(compiler.hooks.watchRun.tapAsync).mock.calls[0][1](
        null,
        callback,
      )
      expect(registerTheme).toHaveBeenCalled()
      vi.mocked(stat).mockImplementation((_, callback) => {
        ;(callback as any)(null, { mtimeMs: 3 })
      })
      expect(registerTheme).toBeCalledTimes(1)
      vi.mocked(compiler.hooks.watchRun.tapAsync).mock.calls[0][1](
        null,
        callback,
      )
      expect(registerTheme).toBeCalledTimes(2)

      vi.mocked(stat).mockImplementation((_, callback) => {
        ;(callback as any)(1)
      })

      plugin.apply(compiler as any)

      vi.mocked(compiler.hooks.watchRun.tapAsync).mock.calls[0][1](
        null,
        callback,
      )

      expect(console.error).toHaveBeenCalledWith(
        'Error checking devup.json:',
        1,
      )
    })
  })

  it('should include lib', () => {
    vi.mocked(readFileSync).mockReturnValue('{"theme": "theme"}')
    vi.mocked(getThemeInterface).mockReturnValue('interfaceCode')
    vi.mocked(getCss).mockReturnValue('css')
    vi.mocked(writeFileSync).mockReturnValue()
    vi.mocked(mkdirSync)

    const plugin = new DevupUIWebpackPlugin({
      include: ['lib'],
    })
    const compiler = {
      options: {
        module: {
          rules: [],
        },
        plugins: [],
      },
      hooks: {
        afterCompile: {
          tap: vi.fn(),
        },
        done: {
          tap: vi.fn(),
        },
        watchRun: {
          tapAsync: vi.fn(),
        },
      },
      webpack: {
        DefinePlugin: vi.fn(),
      },
    } as any
    plugin.apply(compiler)

    expect(writeFileSync).toHaveBeenCalledWith(
      resolve('df', 'devup-ui.css'),
      '',
      {
        encoding: 'utf-8',
      },
    )
    expect(compiler.webpack.DefinePlugin).toHaveBeenCalledWith({
      'process.env.DEVUP_UI_DEFAULT_THEME': JSON.stringify(getDefaultTheme()),
    })
  })
})
