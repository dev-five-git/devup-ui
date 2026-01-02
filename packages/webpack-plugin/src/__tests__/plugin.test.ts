import { join, resolve } from 'node:path'

import type { Mock } from 'bun:test'
import { afterAll, beforeEach, describe, expect, it, mock } from 'bun:test'

mock.module('@devup-ui/wasm', () => ({
  getCss: mock(),
  getDefaultTheme: mock(),
  getThemeInterface: mock(),
  importClassMap: mock(),
  importFileMap: mock(),
  importSheet: mock(),
  registerTheme: mock(),
  setDebug: mock(),
  setPrefix: mock(),
}))
mock.module('node:fs', () => ({
  existsSync: mock(),
  mkdirSync: mock(),
  readFileSync: mock(),
  writeFileSync: mock(),
}))
mock.module('node:fs/promises', () => ({
  mkdir: mock(),
  readFile: mock(),
  stat: mock(),
  writeFile: mock(),
}))

import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { mkdir, readFile, stat, writeFile } from 'node:fs/promises'

import {
  getCss,
  getDefaultTheme,
  getThemeInterface,
  importClassMap,
  importFileMap,
  importSheet,
  registerTheme,
  setDebug,
  setPrefix,
} from '@devup-ui/wasm'

import { DevupUIWebpackPlugin } from '../plugin'

beforeEach(() => {
  ;(getCss as Mock<typeof getCss>).mockReset()
  ;(getDefaultTheme as Mock<typeof getDefaultTheme>).mockReset()
  ;(getThemeInterface as Mock<typeof getThemeInterface>).mockReset()
  ;(importClassMap as Mock<typeof importClassMap>).mockReset()
  ;(importFileMap as Mock<typeof importFileMap>).mockReset()
  ;(importSheet as Mock<typeof importSheet>).mockReset()
  ;(registerTheme as Mock<typeof registerTheme>).mockReset()
  ;(setDebug as Mock<typeof setDebug>).mockReset()
  ;(setPrefix as Mock<typeof setPrefix>).mockReset()
  ;(existsSync as Mock<typeof existsSync>).mockReset()
  ;(mkdirSync as Mock<typeof mkdirSync>).mockReset()
  ;(readFileSync as Mock<typeof readFileSync>).mockReset()
  ;(writeFileSync as Mock<typeof writeFileSync>).mockReset()
  ;(mkdir as Mock<typeof mkdir>).mockReset()
  ;(readFile as Mock<typeof readFile>).mockReset()
  ;(stat as Mock<typeof stat>).mockReset()
  ;(writeFile as Mock<typeof writeFile>).mockReset()
})
afterAll(() => {
  // restore mocks
})
function createCompiler() {
  return {
    options: {
      module: {
        rules: [],
      },
      plugins: [],
    },
    webpack: {
      DefinePlugin: mock(),
    },
    hooks: {
      watchRun: {
        tapPromise: mock(),
      },
      beforeRun: {
        tapPromise: mock(),
      },
      done: {
        tapPromise: mock(),
      },
      afterCompile: {
        tap: mock(),
      },
    },
  } as any
}

describe('devupUIWebpackPlugin', () => {
  console.error = mock()

  it('should apply default options', () => {
    expect(new DevupUIWebpackPlugin({}).options).toEqual({
      include: [],
      package: '@devup-ui/react',
      cssDir: resolve('df', 'devup-ui'),
      devupFile: 'devup.json',
      distDir: 'df',
      watch: false,
      debug: false,
      singleCss: false,
    })
  })

  describe.each(
    createTestMatrix({
      watch: [true, false],
      debug: [true, false],
      singleCss: [true, false],
      include: [['lib'], []],
      package: ['@devup-ui/react', '@devup-ui/core'],
      cssDir: [resolve('df', 'devup-ui'), resolve('df', 'devup-ui-core')],
      devupFile: ['devup.json', 'devup-core.json'],
      distDir: ['df', 'df-core'],
    }),
  )('options', (options) => {
    it('should apply options', () => {
      expect(new DevupUIWebpackPlugin(options).options).toEqual(options)
    })

    it.each(
      createTestMatrix({
        readFile: [{ theme: 'theme' }, { theme: 'theme-core' }, undefined],
        getThemeInterface: ['interfaceCode', ''],
        getCss: ['css', 'css-core'],
      }),
    )('should write data files', async (_options) => {
      ;(readFileSync as Mock<typeof readFileSync>).mockReturnValue(
        JSON.stringify(_options.readFile),
      )
      ;(getThemeInterface as Mock<typeof getThemeInterface>).mockReturnValue(
        _options.getThemeInterface,
      )
      ;(getCss as Mock<typeof getCss>).mockReturnValue(_options.getCss)
      ;(existsSync as Mock<typeof existsSync>).mockReturnValueOnce(
        _options.readFile !== undefined,
      )
      ;(writeFileSync as Mock<typeof writeFileSync>).mockReturnValue()

      const plugin = new DevupUIWebpackPlugin(options)
      await plugin.writeDataFiles()

      if (_options.readFile !== undefined) {
        expect(readFileSync).toHaveBeenCalledWith(options.devupFile, 'utf-8')

        expect(registerTheme).toHaveBeenCalledWith(
          _options.readFile?.theme ?? {},
        )
        expect(getThemeInterface).toHaveBeenCalledWith(
          options.package,
          'CustomColors',
          'DevupThemeTypography',
          'DevupTheme',
        )
        if (_options.getThemeInterface)
          expect(writeFileSync).toHaveBeenCalledWith(
            join(options.distDir, 'theme.d.ts'),
            _options.getThemeInterface,
            {
              encoding: 'utf-8',
            },
          )
        else expect(writeFileSync).toHaveBeenCalledTimes(options.watch ? 1 : 0)
      } else expect(readFile).not.toHaveBeenCalled()
      if (options.watch)
        expect(writeFileSync).toHaveBeenCalledWith(
          join(options.cssDir, 'devup-ui.css'),
          _options.getCss,
        )
      else
        expect(writeFileSync).toHaveBeenCalledTimes(
          _options.getThemeInterface && _options.readFile !== undefined ? 1 : 0,
        )
    })
  })

  it.each(
    createTestMatrix({
      include: [
        {
          input: ['lib'],
          output: new RegExp(
            '(node_modules(?!.*(@devup-ui|lib)([\\/\\\\.]|$)))|(.mdx.[tj]sx?$)',
          ),
        },
        {
          input: [],
          output: new RegExp(
            '(node_modules(?!.*(@devup-ui)([\\/\\\\.]|$)))|(.mdx.[tj]sx?$)',
          ),
        },
        {
          input: ['lib', 'lib2'],
          output: new RegExp(
            '(node_modules(?!.*(@devup-ui|lib|lib2)([\\/\\\\.]|$)))|(.mdx.[tj]sx?$)',
          ),
        },
      ],
    }),
  )('should set include', async (options) => {
    const plugin = new DevupUIWebpackPlugin({
      include: options.include.input,
    })
    ;(existsSync as Mock<typeof existsSync>).mockReturnValue(false)

    const compiler = createCompiler()
    await plugin.apply(compiler)
    expect(compiler.options.module.rules.length).toBe(2)

    expect(compiler.options.module.rules[0].exclude).toEqual(
      options.include.output,
    )
  })

  it.each(
    createTestMatrix({
      debug: [true, false],
    }),
  )('should set debug', async (options) => {
    const plugin = new DevupUIWebpackPlugin(options)

    const compiler = createCompiler()

    await plugin.apply(compiler)
    expect(setDebug).toHaveBeenCalledWith(options.debug)
  })

  it('should reset data files when load error', async () => {
    const plugin = new DevupUIWebpackPlugin({
      watch: true,
    })
    const compiler = createCompiler()
    ;(readFileSync as Mock<typeof readFileSync>).mockImplementation(() => {
      throw new Error('error')
    })
    ;(stat as Mock<typeof stat>).mockReturnValue({
      mtimeMs: 1,
    } as any)
    ;(existsSync as Mock<typeof existsSync>).mockReturnValue(true)
    plugin.apply(compiler as any)
    await compiler.hooks.watchRun.tapPromise.mock.calls[0][1]()
    expect(importSheet).toHaveBeenCalledWith({})
    expect(importClassMap).toHaveBeenCalledWith({})
    expect(importFileMap).toHaveBeenCalledWith({})
  })

  it.each(
    createTestMatrix({
      watch: [true, false],
      existsDevupFile: [true, false],
      existsDistDir: [true, false],
      existsSheetFile: [true, false],
      existsClassMapFile: [true, false],
      existsFileMapFile: [true, false],
      existsCssDir: [true, false],
    }),
  )('should apply', async (options) => {
    const plugin = new DevupUIWebpackPlugin({
      watch: options.watch,
    })
    const compiler = createCompiler()

    ;(existsSync as Mock<typeof existsSync>).mockImplementation((path) => {
      if (path === plugin.options.devupFile) return options.existsDevupFile
      if (path === plugin.options.distDir) return options.existsDistDir
      if (path === plugin.options.cssDir) return options.existsCssDir
      if (path === join(plugin.options.distDir, 'sheet.json'))
        return options.existsSheetFile
      if (path === join(plugin.options.distDir, 'classMap.json'))
        return options.existsClassMapFile
      if (path === join(plugin.options.distDir, 'fileMap.json'))
        return options.existsFileMapFile
      return false
    })
    ;(getDefaultTheme as Mock<typeof getDefaultTheme>).mockReturnValue(
      'defaultTheme',
    )
    ;(stat as Mock<typeof stat>).mockResolvedValueOnce({
      mtimeMs: 1,
    } as any)
    ;(stat as Mock<typeof stat>).mockResolvedValueOnce({
      mtimeMs: 2,
    } as any)
    ;(readFileSync as Mock<typeof readFileSync>).mockImplementation(
      (path: string) => {
        if (
          path === join(plugin.options.distDir, 'sheet.json') &&
          options.existsSheetFile
        )
          return '{"sheet": "sheet"}'
        if (
          path === join(plugin.options.distDir, 'classMap.json') &&
          options.existsClassMapFile
        )
          return '{"classMap": "classMap"}'
        if (
          path === join(plugin.options.distDir, 'fileMap.json') &&
          options.existsFileMapFile
        )
          return '{"fileMap": "fileMap"}'
        return '{}'
      },
    )

    plugin.apply(compiler)

    if (options.existsDistDir)
      expect(mkdirSync).not.toHaveBeenCalledWith(plugin.options.distDir, {
        recursive: true,
      })
    else
      expect(mkdirSync).toHaveBeenCalledWith(plugin.options.distDir, {
        recursive: true,
      })
    expect(writeFileSync).toHaveBeenCalledWith(
      join(plugin.options.distDir, '.gitignore'),
      '*',
      'utf-8',
    )
    if (options.watch) {
      if (options.existsSheetFile)
        expect(importSheet).toHaveBeenCalledWith(
          JSON.parse('{"sheet": "sheet"}'),
        )
      if (options.existsClassMapFile)
        expect(importClassMap).toHaveBeenCalledWith(
          JSON.parse('{"classMap": "classMap"}'),
        )
      if (options.existsFileMapFile)
        expect(importFileMap).toHaveBeenCalledWith(
          JSON.parse('{"fileMap": "fileMap"}'),
        )
      expect(compiler.hooks.watchRun.tapPromise).toHaveBeenCalled()

      await compiler.hooks.watchRun.tapPromise.mock.calls[0][1]()
      if (options.existsDevupFile) {
        expect(stat).toHaveBeenCalledWith(plugin.options.devupFile)
        await compiler.hooks.watchRun.tapPromise.mock.calls[0][1]()
      } else {
        expect(stat).not.toHaveBeenCalled()
      }
    } else expect(compiler.hooks.watchRun.tapPromise).not.toHaveBeenCalled()
    if (options.existsDevupFile) {
      expect(compiler.hooks.afterCompile.tap).toHaveBeenCalled()
      const add = mock()
      compiler.hooks.afterCompile.tap.mock.calls[0][1]({
        fileDependencies: {
          add,
        },
      })
      expect(add).toHaveBeenCalledWith(resolve(plugin.options.devupFile))
    } else expect(compiler.hooks.afterCompile.tap).not.toHaveBeenCalled()
    if (options.existsCssDir) {
      expect(mkdir).not.toHaveBeenCalledWith(plugin.options.cssDir, {
        recursive: true,
      })
    } else {
      expect(mkdirSync).toHaveBeenCalledWith(plugin.options.cssDir, {
        recursive: true,
      })
    }

    expect(compiler.webpack.DefinePlugin).toHaveBeenCalledWith({
      'process.env.DEVUP_UI_DEFAULT_THEME': JSON.stringify(getDefaultTheme()),
    })

    if (!options.watch) {
      expect(compiler.hooks.done.tapPromise).toHaveBeenCalled()
      compiler.hooks.done.tapPromise.mock.calls[0][1]({
        hasErrors: () => true,
      })
      expect(writeFile).not.toHaveBeenCalledWith(
        join(plugin.options.cssDir, 'devup-ui.css'),
        getCss(null, true),
        'utf-8',
      )

      await compiler.hooks.done.tapPromise.mock.calls[0][1]({
        hasErrors: () => false,
      })
      expect(writeFile).toHaveBeenCalledWith(
        join(plugin.options.cssDir, 'devup-ui.css'),
        getCss(null, true),
        'utf-8',
      )
    } else {
      expect(compiler.hooks.done.tapPromise).not.toHaveBeenCalled()
    }
  })

  it('should call setPrefix when prefix option is provided', async () => {
    const plugin = new DevupUIWebpackPlugin({ prefix: 'my-prefix' })
    const compiler = createCompiler()
    ;(existsSync as Mock<typeof existsSync>).mockReturnValue(false)
    plugin.apply(compiler)
    expect(setPrefix).toHaveBeenCalledWith('my-prefix')
  })
})
