import * as fs from 'node:fs'
import * as fsPromises from 'node:fs/promises'
import { join, resolve } from 'node:path'

import * as pluginUtils from '@devup-ui/plugin-utils'
import * as wasm from '@devup-ui/wasm'
import {
  afterEach,
  beforeEach,
  describe,
  expect,
  it,
  mock,
  spyOn,
} from 'bun:test'

import { DevupUIWebpackPlugin } from '../plugin'

let codeExtractSpy: ReturnType<typeof spyOn>
let getCssSpy: ReturnType<typeof spyOn>
let getDefaultThemeSpy: ReturnType<typeof spyOn>
let getThemeInterfaceSpy: ReturnType<typeof spyOn>
let importClassMapSpy: ReturnType<typeof spyOn>
let importFileMapSpy: ReturnType<typeof spyOn>
let importSheetSpy: ReturnType<typeof spyOn>
let registerThemeSpy: ReturnType<typeof spyOn>
let setDebugSpy: ReturnType<typeof spyOn>
let setPrefixSpy: ReturnType<typeof spyOn>
let loadDevupConfigSyncSpy: ReturnType<typeof spyOn>
let existsSyncSpy: ReturnType<typeof spyOn>
let mkdirSyncSpy: ReturnType<typeof spyOn>
let readFileSyncSpy: ReturnType<typeof spyOn>
let writeFileSyncSpy: ReturnType<typeof spyOn>
let mkdirSpy: ReturnType<typeof spyOn>
let readFileSpy: ReturnType<typeof spyOn>
let statSpy: ReturnType<typeof spyOn>
let writeFileSpy: ReturnType<typeof spyOn>

beforeEach(() => {
  codeExtractSpy = spyOn(wasm, 'codeExtract').mockImplementation(
    (_path: string, contents: string) =>
      ({
        css: '',
        code: contents,
        cssFile: '',
        map: undefined,
        updatedBaseStyle: false,
        free: mock(),
        [Symbol.dispose]: mock(),
      }) as any,
  )
  getCssSpy = spyOn(wasm, 'getCss').mockReturnValue('')
  getDefaultThemeSpy = spyOn(wasm, 'getDefaultTheme').mockReturnValue(undefined)
  getThemeInterfaceSpy = spyOn(wasm, 'getThemeInterface').mockReturnValue('')
  importClassMapSpy = spyOn(wasm, 'importClassMap').mockReturnValue(undefined)
  importFileMapSpy = spyOn(wasm, 'importFileMap').mockReturnValue(undefined)
  importSheetSpy = spyOn(wasm, 'importSheet').mockReturnValue(undefined)
  registerThemeSpy = spyOn(wasm, 'registerTheme').mockReturnValue(undefined)
  setDebugSpy = spyOn(wasm, 'setDebug').mockReturnValue(undefined)
  setPrefixSpy = spyOn(wasm, 'setPrefix').mockReturnValue(undefined)
  loadDevupConfigSyncSpy = spyOn(
    pluginUtils,
    'loadDevupConfigSync',
  ).mockReturnValue({})
  existsSyncSpy = spyOn(fs, 'existsSync').mockReturnValue(false)
  mkdirSyncSpy = spyOn(fs, 'mkdirSync').mockReturnValue(undefined)
  readFileSyncSpy = spyOn(fs, 'readFileSync').mockReturnValue('{}')
  writeFileSyncSpy = spyOn(fs, 'writeFileSync').mockReturnValue(undefined)
  mkdirSpy = spyOn(fsPromises, 'mkdir').mockResolvedValue(undefined)
  readFileSpy = spyOn(fsPromises, 'readFile').mockResolvedValue('{}')
  statSpy = spyOn(fsPromises, 'stat').mockResolvedValue({ mtimeMs: 0 } as any)
  writeFileSpy = spyOn(fsPromises, 'writeFile').mockResolvedValue(undefined)
})

afterEach(() => {
  codeExtractSpy.mockRestore()
  getCssSpy.mockRestore()
  getDefaultThemeSpy.mockRestore()
  getThemeInterfaceSpy.mockRestore()
  importClassMapSpy.mockRestore()
  importFileMapSpy.mockRestore()
  importSheetSpy.mockRestore()
  registerThemeSpy.mockRestore()
  setDebugSpy.mockRestore()
  setPrefixSpy.mockRestore()
  loadDevupConfigSyncSpy.mockRestore()
  existsSyncSpy.mockRestore()
  mkdirSyncSpy.mockRestore()
  readFileSyncSpy.mockRestore()
  writeFileSyncSpy.mockRestore()
  mkdirSpy.mockRestore()
  readFileSpy.mockRestore()
  statSpy.mockRestore()
  writeFileSpy.mockRestore()
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
      loadDevupConfigSyncSpy.mockReturnValue(
        _options.readFile !== undefined
          ? { theme: _options.readFile.theme }
          : {},
      )
      getThemeInterfaceSpy.mockReturnValue(_options.getThemeInterface)
      getCssSpy.mockReturnValue(_options.getCss)
      writeFileSyncSpy.mockReturnValue(undefined)

      const plugin = new DevupUIWebpackPlugin(options)
      await plugin.writeDataFiles()

      expect(loadDevupConfigSyncSpy).toHaveBeenCalledWith(options.devupFile)
      expect(registerThemeSpy).toHaveBeenCalledWith(
        _options.readFile?.theme ?? {},
      )
      expect(getThemeInterfaceSpy).toHaveBeenCalledWith(
        options.package,
        'CustomColors',
        'DevupThemeTypography',
        'DevupTheme',
      )
      if (_options.getThemeInterface)
        expect(writeFileSyncSpy).toHaveBeenCalledWith(
          join(options.distDir, 'theme.d.ts'),
          _options.getThemeInterface,
          {
            encoding: 'utf-8',
          },
        )
      else expect(writeFileSyncSpy).toHaveBeenCalledTimes(options.watch ? 1 : 0)

      if (options.watch)
        expect(writeFileSyncSpy).toHaveBeenCalledWith(
          join(options.cssDir, 'devup-ui.css'),
          _options.getCss,
        )
      else
        expect(writeFileSyncSpy).toHaveBeenCalledTimes(
          _options.getThemeInterface ? 1 : 0,
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
    existsSyncSpy.mockReturnValue(false)

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
    expect(setDebugSpy).toHaveBeenCalledWith(options.debug)
  })

  it('should reset data files when load error', async () => {
    const plugin = new DevupUIWebpackPlugin({
      watch: true,
    })
    const compiler = createCompiler()
    readFileSyncSpy.mockImplementation(() => {
      throw new Error('error')
    })
    statSpy.mockReturnValue({
      mtimeMs: 1,
    } as any)
    existsSyncSpy.mockReturnValue(true)
    plugin.apply(compiler as any)
    await compiler.hooks.watchRun.tapPromise.mock.calls[0][1]()
    expect(importSheetSpy).toHaveBeenCalledWith({})
    expect(importClassMapSpy).toHaveBeenCalledWith({})
    expect(importFileMapSpy).toHaveBeenCalledWith({})
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

    existsSyncSpy.mockImplementation((path: string) => {
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
    getDefaultThemeSpy.mockReturnValue('defaultTheme')
    statSpy.mockResolvedValueOnce({
      mtimeMs: 1,
    } as any)
    statSpy.mockResolvedValueOnce({
      mtimeMs: 2,
    } as any)
    readFileSyncSpy.mockImplementation((path: string) => {
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
    })

    plugin.apply(compiler)

    if (options.existsDistDir)
      expect(mkdirSyncSpy).not.toHaveBeenCalledWith(plugin.options.distDir, {
        recursive: true,
      })
    else
      expect(mkdirSyncSpy).toHaveBeenCalledWith(plugin.options.distDir, {
        recursive: true,
      })
    expect(writeFileSyncSpy).toHaveBeenCalledWith(
      join(plugin.options.distDir, '.gitignore'),
      '*',
      'utf-8',
    )
    if (options.watch) {
      if (options.existsSheetFile)
        expect(importSheetSpy).toHaveBeenCalledWith(
          JSON.parse('{"sheet": "sheet"}'),
        )
      if (options.existsClassMapFile)
        expect(importClassMapSpy).toHaveBeenCalledWith(
          JSON.parse('{"classMap": "classMap"}'),
        )
      if (options.existsFileMapFile)
        expect(importFileMapSpy).toHaveBeenCalledWith(
          JSON.parse('{"fileMap": "fileMap"}'),
        )
      expect(compiler.hooks.watchRun.tapPromise).toHaveBeenCalled()

      await compiler.hooks.watchRun.tapPromise.mock.calls[0][1]()
      if (options.existsDevupFile) {
        expect(statSpy).toHaveBeenCalledWith(plugin.options.devupFile)
        await compiler.hooks.watchRun.tapPromise.mock.calls[0][1]()
      } else {
        expect(statSpy).not.toHaveBeenCalled()
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
      expect(mkdirSpy).not.toHaveBeenCalledWith(plugin.options.cssDir, {
        recursive: true,
      })
    } else {
      expect(mkdirSyncSpy).toHaveBeenCalledWith(plugin.options.cssDir, {
        recursive: true,
      })
    }

    expect(compiler.webpack.DefinePlugin).toHaveBeenCalledWith({
      'process.env.DEVUP_UI_DEFAULT_THEME': JSON.stringify(
        getDefaultThemeSpy.mock.results[0]?.value,
      ),
    })

    if (!options.watch) {
      expect(compiler.hooks.done.tapPromise).toHaveBeenCalled()
      compiler.hooks.done.tapPromise.mock.calls[0][1]({
        hasErrors: () => true,
      })
      expect(writeFileSpy).not.toHaveBeenCalledWith(
        join(plugin.options.cssDir, 'devup-ui.css'),
        getCssSpy.mock.results[0]?.value,
        'utf-8',
      )

      await compiler.hooks.done.tapPromise.mock.calls[0][1]({
        hasErrors: () => false,
      })
      expect(writeFileSpy).toHaveBeenCalledWith(
        join(plugin.options.cssDir, 'devup-ui.css'),
        getCssSpy.mock.results[0]?.value,
        'utf-8',
      )
    } else {
      expect(compiler.hooks.done.tapPromise).not.toHaveBeenCalled()
    }
  })

  it('should call setPrefix when prefix option is provided', async () => {
    const plugin = new DevupUIWebpackPlugin({ prefix: 'my-prefix' })
    const compiler = createCompiler()
    existsSyncSpy.mockReturnValue(false)
    plugin.apply(compiler)
    expect(setPrefixSpy).toHaveBeenCalledWith('my-prefix')
  })
})
