import {
  existsSync,
  mkdirSync,
  readFileSync,
  stat,
  writeFileSync,
} from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import { getCss, getThemeInterface, registerTheme } from '@devup-ui/wasm'

import { DevupUIWebpackPlugin } from '../plugin'

vi.mock('@devup-ui/wasm')
vi.mock('node:fs')

const _filename = fileURLToPath(import.meta.url)
const _dirname = resolve(dirname(_filename), '..')
beforeEach(() => {
  vi.resetAllMocks()
})

describe('devupUIPlugin', () => {
  it('should apply default options', () => {
    import.meta.resolve = vi.fn().mockReturnValue('resolved')
    expect(new DevupUIWebpackPlugin({}).options).toEqual({
      package: '@devup-ui/react',
      cssFile: join(_dirname, 'devup-ui.css'),
      devupPath: 'devup.json',
      interfacePath: '.df',
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
      }).options,
    ).toEqual({
      package: 'new-package',
      cssFile: 'new-css-file',
      devupPath: 'new-devup-path',
      interfacePath: 'new-interface-path',
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
    )
    expect(mkdirSync).toHaveBeenCalledWith('.df')
    expect(writeFileSync).toHaveBeenCalledWith(
      join('.df', 'theme.d.ts'),
      'interfaceCode',
      {
        encoding: 'utf-8',
      },
    )
    expect(writeFileSync).toHaveBeenCalledWith(
      join(_dirname, 'devup-ui.css'),
      'css',
      {
        encoding: 'utf-8',
      },
    )
  })
  it('should watch devup.json', () => {
    vi.mocked(readFileSync).mockReturnValue('{"theme": "theme"}')
    vi.mocked(existsSync).mockReturnValue(true)
    const plugin = new DevupUIWebpackPlugin({})
    const compiler = {
      options: {
        module: {
          rules: [],
        },
      },
      hooks: {
        afterCompile: {
          tap: vi.fn(),
        },
        watchRun: {
          tapAsync: vi.fn(),
        },
      },
    }
    plugin.apply(compiler as any)
    expect(compiler.hooks.afterCompile.tap).toHaveBeenCalled()
    expect(compiler.hooks.watchRun.tapAsync).toHaveBeenCalled()
  })

  it('should catch error', () => {
    vi.mocked(existsSync).mockReturnValue(true)
    vi.mocked(stat).mockImplementation((_, callback) => {
      ;(callback as any)(new Error('error'), null)
    })
    vi.mocked(readFileSync).mockImplementation(() => {
      throw new Error('error')
    })
    console.error = vi.fn()
    const plugin = new DevupUIWebpackPlugin({
      devupPath: 'custom-devup.json',
    })
    const compiler = {
      options: {
        module: {
          rules: [],
        },
      },
      hooks: {
        afterCompile: {
          tap: vi.fn(),
        },
        watchRun: {
          tapAsync: vi.fn(),
        },
      },
    }
    plugin.apply(compiler as any)
    expect(console.error).toHaveBeenCalledWith(new Error('error'))
    // asyncCompile
    const add = vi.fn()
    vi.mocked(compiler.hooks.afterCompile.tap).mock.calls[0][1]({
      fileDependencies: {
        add,
      },
    })
    expect(add).toHaveBeenCalledWith(resolve('custom-devup.json'))

    // watchRun
    const callback = vi.fn()
    vi.mocked(compiler.hooks.watchRun.tapAsync).mock.calls[0][1](null, callback)
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

    vi.mocked(compiler.hooks.watchRun.tapAsync).mock.calls[0][1](null, callback)
    expect(registerTheme).toHaveBeenCalled()
    vi.mocked(stat).mockImplementation((_, callback) => {
      ;(callback as any)(null, { mtimeMs: 3 })
    })
    expect(registerTheme).toBeCalledTimes(1)
    vi.mocked(compiler.hooks.watchRun.tapAsync).mock.calls[0][1](null, callback)
    expect(registerTheme).toBeCalledTimes(2)
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
      },
      hooks: {
        afterCompile: {
          tap: vi.fn(),
        },
        watchRun: {
          tapAsync: vi.fn(),
        },
      },
    } as any)

    expect(writeFileSync).toHaveBeenCalledWith('css', '', {
      encoding: 'utf-8',
    })
  })
})
