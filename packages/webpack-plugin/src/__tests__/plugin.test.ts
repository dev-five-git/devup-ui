import type { Stats } from 'node:fs'
import * as fs from 'node:fs'
import * as fsPromises from 'node:fs/promises'
import { join, relative, resolve } from 'node:path'

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
import type { Compiler } from 'webpack'

import { DevupUIWebpackPlugin } from '../plugin'
import { servedCss } from '../served-css'

type CodeExtractResult = ReturnType<typeof wasm.codeExtract>
interface MockCompiler {
  options: {
    module: { rules: unknown[] }
    plugins: unknown[]
    entry?: unknown
    context?: string
    target?: string
  }
  watchMode?: boolean
  webpack: {
    DefinePlugin: ReturnType<typeof mock>
    Compilation: { PROCESS_ASSETS_STAGE_REPORT: number }
  }
  hooks: {
    watchRun: { tapPromise: ReturnType<typeof mock> }
    beforeRun: { tapPromise: ReturnType<typeof mock> }
    done: { tapPromise: ReturnType<typeof mock> }
    afterCompile: { tap: ReturnType<typeof mock> }
    run: { tap: ReturnType<typeof mock> }
    thisCompilation: { tap: ReturnType<typeof mock> }
    shutdown: { tap: ReturnType<typeof mock> }
  }
}

function createCodeExtractResult(
  contents: string,
  overrides: Partial<CodeExtractResult> = {},
): CodeExtractResult {
  return {
    css: '',
    code: contents,
    cssFile: '',
    map: undefined,
    updatedBaseStyle: false,
    free: mock(),
    [Symbol.dispose]: mock(),
    ...overrides,
  } as unknown as CodeExtractResult
}

function createStats(mtimeMs: number): Stats {
  return { mtimeMs } as unknown as Stats
}

function asCompiler(compiler: MockCompiler): Compiler {
  return compiler as unknown as Compiler
}

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
let setNamingRootSpy: ReturnType<typeof spyOn>
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
    (_path: string, contents: string) => createCodeExtractResult(contents),
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
  setNamingRootSpy = spyOn(wasm, 'setNamingRoot').mockReturnValue(undefined)
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
  statSpy = spyOn(fsPromises, 'stat').mockResolvedValue(createStats(0))
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
  setNamingRootSpy.mockRestore()
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

function createCompiler(): MockCompiler {
  return {
    options: {
      module: {
        rules: [],
      },
      plugins: [],
    },
    webpack: {
      DefinePlugin: mock(),
      Compilation: { PROCESS_ASSETS_STAGE_REPORT: 5000 },
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
      run: {
        tap: mock(),
      },
      thisCompilation: {
        tap: mock(),
      },
      shutdown: {
        tap: mock(),
      },
    },
  }
}

describe('devupUIWebpackPlugin', () => {
  it('sets the same context naming root once in every compiler', () => {
    for (const target of ['web', 'node']) {
      const compiler = createCompiler()
      compiler.options.context = '/naming-project'
      compiler.options.target = target
      new DevupUIWebpackPlugin({}).apply(asCompiler(compiler))
    }
    expect(setNamingRootSpy.mock.calls).toEqual([
      ['/naming-project', process.cwd()],
      ['/naming-project', process.cwd()],
    ])
  })
  console.error = mock()

  describe('deterministic file numbering', () => {
    it('numbers the files the scan finds and ends the build at shutdown', () => {
      const collectSpy = spyOn(pluginUtils, 'collectNumberedFiles')
      const seedSpy = spyOn(wasm, 'seedFileMap').mockReturnValue(undefined)
      const resetSpy = spyOn(wasm, 'resetBuildState').mockReturnValue(undefined)
      try {
        collectSpy.mockImplementation((options: any) => {
          expect(options.toId(resolve('src', 'a.tsx'))).toBe('src/a.tsx')
          return ['src/a.tsx', 'src/b.tsx']
        })
        const compiler = createCompiler()
        new DevupUIWebpackPlugin({ include: ['@acme/ui'] }).apply(
          asCompiler(compiler),
        )
        expect(seedSpy).toHaveBeenCalledWith(['src/a.tsx', 'src/b.tsx'])
        expect(collectSpy.mock.calls[0][0]).toMatchObject({
          roots: [resolve('src')],
          include: ['@acme/ui'],
        })
        compiler.hooks.shutdown.tap.mock.calls[0][1]()
        compiler.hooks.shutdown.tap.mock.calls[0][1]()
        collectSpy.mockImplementation(() => {
          throw new Error('scan boom')
        })
        new DevupUIWebpackPlugin({}).apply(asCompiler(createCompiler()))
      } finally {
        collectSpy.mockRestore()
        seedSpy.mockRestore()
        resetSpy.mockRestore()
      }
    })
  })
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
        'CustomLength',
        'CustomShadows',
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
            '(node_modules(?!.*(@devup-ui|@devup-editor|lib)([\\/\\\\.]|$)))|(.mdx.[tj]sx?$)',
          ),
        },
        {
          input: [],
          output: new RegExp(
            '(node_modules(?!.*(@devup-ui|@devup-editor)([\\/\\\\.]|$)))|(.mdx.[tj]sx?$)',
          ),
        },
        {
          input: ['lib', 'lib2'],
          output: new RegExp(
            '(node_modules(?!.*(@devup-ui|@devup-editor|lib|lib2)([\\/\\\\.]|$)))|(.mdx.[tj]sx?$)',
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
    await plugin.apply(asCompiler(compiler))
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

    await plugin.apply(asCompiler(compiler))
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
    statSpy.mockReturnValue(createStats(1))
    existsSyncSpy.mockReturnValue(true)
    plugin.apply(asCompiler(compiler))
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
    statSpy.mockResolvedValueOnce(createStats(1))
    statSpy.mockResolvedValueOnce(createStats(2))
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

    plugin.apply(asCompiler(compiler))

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
    plugin.apply(asCompiler(compiler))
    expect(setPrefixSpy).toHaveBeenCalledWith('my-prefix')
  })

  describe('collapse + atomHoist pre-pass', () => {
    let buildCanonicalMapSpy: ReturnType<typeof spyOn>
    let computeFileReachSpy: ReturnType<typeof spyOn>
    let importCanonicalMapSpy: ReturnType<typeof spyOn>
    let importFileRoutesSpy: ReturnType<typeof spyOn>
    let setAtomHoistSpy: ReturnType<typeof spyOn>
    let computeReachableFilesSpy: ReturnType<typeof spyOn>

    beforeEach(() => {
      buildCanonicalMapSpy = spyOn(
        pluginUtils,
        'buildCanonicalMap',
      ).mockReturnValue({})
      computeFileReachSpy = spyOn(
        pluginUtils,
        'computeFileReach',
      ).mockReturnValue({})
      importCanonicalMapSpy = spyOn(wasm, 'importCanonicalMap').mockReturnValue(
        undefined,
      )
      importFileRoutesSpy = spyOn(wasm, 'importFileRoutes').mockReturnValue(
        undefined,
      )
      setAtomHoistSpy = spyOn(wasm, 'setAtomHoist').mockReturnValue(undefined)
      // Default: no source files so pre-warm is a no-op unless a test opts in.
      computeReachableFilesSpy = spyOn(
        pluginUtils,
        'computeReachableFiles',
      ).mockReturnValue([])
    })

    afterEach(() => {
      buildCanonicalMapSpy.mockRestore()
      computeFileReachSpy.mockRestore()
      importCanonicalMapSpy.mockRestore()
      importFileRoutesSpy.mockRestore()
      setAtomHoistSpy.mockRestore()
      computeReachableFilesSpy.mockRestore()
    })

    it('runs single-importer collapse even when atomHoist is unset (always-on), without hoisting', () => {
      // Constraint: single-importer collapse must ALWAYS be on. The canonical
      // map is built + imported unconditionally; only atom HOISTING is gated.
      buildCanonicalMapSpy.mockReturnValue({
        'src/child.tsx': 'src/parent.tsx',
      })
      const plugin = new DevupUIWebpackPlugin({})
      plugin.apply(asCompiler(createCompiler()))
      expect(buildCanonicalMapSpy).toHaveBeenCalledWith(
        expect.objectContaining({ keyBy: 'cwd-relative' }),
      )
      expect(importCanonicalMapSpy).toHaveBeenCalled()
      // atom hoisting stays off without atomHoist
      expect(computeFileReachSpy).not.toHaveBeenCalled()
      expect(setAtomHoistSpy).not.toHaveBeenCalled()
      expect(importFileRoutesSpy).not.toHaveBeenCalled()
    })

    it('pre-warms the extractor over the files the entries reach in build mode', () => {
      // Without pre-warm, webpack builds each shared stylesheet ONCE at first
      // import — before later files are extracted — so their styles are
      // dropped. Pre-warming the sheet makes getCss complete from the first
      // css-loader call.
      buildCanonicalMapSpy.mockReturnValue({
        'src/child.tsx': 'src/parent.tsx',
      })
      computeReachableFilesSpy.mockReturnValue([
        resolve(process.cwd(), 'src', 'parent.tsx'),
        resolve(process.cwd(), 'src', 'child.tsx'),
      ])
      readFileSyncSpy.mockReturnValue('source')
      const setModuleResolverSpy = spyOn(
        wasm,
        'setModuleResolver',
      ).mockReturnValue(undefined)
      const plugin = new DevupUIWebpackPlugin({
        package: '@devup-ui/react',
        singleCss: true,
      })
      const compiler = createCompiler()
      compiler.options.entry = {
        main: { import: ['./src/parent', './src/other'] },
        worker: {},
      }
      compiler.options.context = '/project'
      plugin.apply(asCompiler(compiler))
      expect(computeReachableFilesSpy).toHaveBeenCalledWith({
        srcDir: resolve(process.cwd(), 'src'),
        tsconfigPath: resolve(process.cwd(), 'tsconfig.json'),
        entries: [
          resolve('/project', './src/parent'),
          resolve('/project', './src/other'),
        ],
      })
      // The loader's resolver is registered before the first extraction, so
      // imported constants inline as they do in the loader
      expect(setModuleResolverSpy.mock.invocationCallOrder[0]).toBeLessThan(
        codeExtractSpy.mock.invocationCallOrder[0]!,
      )
      const resolveModule = setModuleResolverSpy.mock.calls[0]![0] as (
        specifier: string,
        importer: string,
      ) => { path: string } | undefined
      expect(resolveModule('./plugin.test', import.meta.path)?.path).toBe(
        relative(process.cwd(), import.meta.path).replaceAll('\\', '/'),
      )
      setModuleResolverSpy.mockRestore()
      expect(codeExtractSpy).toHaveBeenCalledTimes(2)
      expect(codeExtractSpy).toHaveBeenCalledWith(
        'src/parent.tsx',
        'source',
        '@devup-ui/react',
        expect.any(String),
        true,
        false,
        true,
        expect.anything(),
      )
      expect(codeExtractSpy).toHaveBeenCalledWith(
        'src/child.tsx',
        'source',
        '@devup-ui/react',
        expect.any(String),
        true,
        false,
        true,
        expect.anything(),
      )
    })

    it('pre-warms without collapse, from no entries when they are a function', () => {
      buildCanonicalMapSpy.mockReturnValue({})
      const compiler = createCompiler()
      compiler.options.entry = () => ({})
      new DevupUIWebpackPlugin({}).apply(asCompiler(compiler))
      expect(computeReachableFilesSpy).toHaveBeenCalledWith(
        expect.objectContaining({ entries: [] }),
      )
    })

    it('skips pre-warm in watch mode (race only affects one-shot builds)', () => {
      buildCanonicalMapSpy.mockReturnValue({
        'src/child.tsx': 'src/parent.tsx',
      })
      computeReachableFilesSpy.mockReturnValue([
        resolve(process.cwd(), 'src', 'parent.tsx'),
      ])
      readFileSyncSpy.mockReturnValue('source')
      const plugin = new DevupUIWebpackPlugin({ watch: true })
      plugin.apply(asCompiler(createCompiler()))
      expect(codeExtractSpy).not.toHaveBeenCalled()
    })

    it('swallows pre-warm errors (extraction failure does not break apply)', () => {
      buildCanonicalMapSpy.mockReturnValue({
        'src/child.tsx': 'src/parent.tsx',
      })
      computeReachableFilesSpy.mockReturnValue([
        resolve(process.cwd(), 'src', 'parent.tsx'),
      ])
      readFileSyncSpy.mockReturnValue('source')
      codeExtractSpy.mockImplementation(() => {
        throw new Error('extract boom')
      })
      const plugin = new DevupUIWebpackPlugin({})
      // apply must still complete without throwing
      plugin.apply(asCompiler(createCompiler()))
      expect(codeExtractSpy).toHaveBeenCalled()
    })

    it('composes collapse + hoist and folds reach onto the canonical bucket', () => {
      buildCanonicalMapSpy.mockReturnValue({
        'src/child.tsx': 'src/parent.tsx',
        'src/glob.tsx': '@global',
      })
      computeFileReachSpy.mockReturnValue({
        'src/parent.tsx': [0, 1],
        'src/child.tsx': [0],
        'src/glob.tsx': [0, 1],
        'src/r1.tsx': [1],
      })
      const plugin = new DevupUIWebpackPlugin({ atomHoist: 2 })
      plugin.apply(asCompiler(createCompiler()))
      // collapse runs with cwd-relative keys (webpack loader passes relative path)
      expect(buildCanonicalMapSpy).toHaveBeenCalledWith(
        expect.objectContaining({ keyBy: 'cwd-relative' }),
      )
      expect(importCanonicalMapSpy).toHaveBeenCalled()
      // reach folded by bucket: child -> parent, @global skipped
      expect(importFileRoutesSpy).toHaveBeenCalledWith({
        'src/parent.tsx': [0, 1],
        'src/r1.tsx': [1],
      })
      expect(setAtomHoistSpy).toHaveBeenCalledWith(2)
    })

    it('clamps the threshold to a minimum of 2', () => {
      computeFileReachSpy.mockReturnValue({
        'src/a.tsx': [0],
        'src/b.tsx': [1],
      })
      const plugin = new DevupUIWebpackPlugin({ atomHoist: 1 })
      plugin.apply(asCompiler(createCompiler()))
      expect(setAtomHoistSpy).toHaveBeenCalledWith(2)
    })

    it('stays off when fewer than two routes are reachable', () => {
      computeFileReachSpy.mockReturnValue({ 'src/a.tsx': [0] })
      const plugin = new DevupUIWebpackPlugin({ atomHoist: 2 })
      plugin.apply(asCompiler(createCompiler()))
      expect(setAtomHoistSpy).not.toHaveBeenCalled()
      expect(importFileRoutesSpy).not.toHaveBeenCalled()
    })

    it('swallows pre-pass errors (atom hoisting stays off)', () => {
      buildCanonicalMapSpy.mockImplementation(() => {
        throw new Error('boom')
      })
      const plugin = new DevupUIWebpackPlugin({ atomHoist: 2 })
      // apply must still complete without throwing
      plugin.apply(asCompiler(createCompiler()))
      expect(setAtomHoistSpy).not.toHaveBeenCalled()
    })

    it('configures atom hoisting BEFORE registering loader rules', () => {
      computeFileReachSpy.mockReturnValue({
        'src/a.tsx': [0],
        'src/b.tsx': [1],
      })
      const compiler = createCompiler()
      let rulesLenAtSetAtomHoist = -1
      setAtomHoistSpy.mockImplementation(() => {
        rulesLenAtSetAtomHoist = compiler.options.module.rules.length
      })
      const plugin = new DevupUIWebpackPlugin({ atomHoist: 2 })
      plugin.apply(asCompiler(compiler))
      // pre-pass must run before module rules are pushed (single WASM instance)
      expect(rulesLenAtSetAtomHoist).toBe(0)
      expect(compiler.options.module.rules.length).toBeGreaterThan(0)
    })
  })

  describe('another pass for stylesheets built too early', () => {
    function compile(watch = false) {
      const compiler = createCompiler()
      const plugin = new DevupUIWebpackPlugin({ watch })
      plugin.apply(asCompiler(compiler))
      const taps: Record<string, (...args: unknown[]) => unknown> = {}
      const tap =
        (hook: string) => (_: unknown, fn: (...args: unknown[]) => unknown) => {
          taps[hook] = fn
        }
      const compilation = {
        assets: { 'main.js': {}, 'main.css': {} },
        deleteAsset: mock(),
        hooks: {
          finishModules: { tap: tap('finishModules') },
          processAssets: { tap: tap('processAssets') },
          needAdditionalPass: { tap: tap('needAdditionalPass') },
        },
      }
      const run = compiler.hooks.run.tap.mock.calls[0]?.[1] as () => void
      const start = () => {
        ;(
          compiler.hooks.thisCompilation.tap.mock.calls[0]![1] as (
            compilation: object,
          ) => void
        )(compilation)
      }
      return { compiler, plugin, compilation, taps, run, start }
    }

    it('taps nothing in watch mode', () => {
      const { compiler } = compile(true)
      expect(compiler.hooks.run.tap).not.toHaveBeenCalled()
      expect(compiler.hooks.thisCompilation.tap).not.toHaveBeenCalled()
    })

    it('writes the shared base to disk when it does not hold it yet', () => {
      getCssSpy.mockReturnValue('base')
      existsSyncSpy.mockReturnValue(true)
      readFileSyncSpy.mockReturnValue('base')
      const { plugin, start } = compile()
      writeFileSyncSpy.mockClear()
      start()
      expect(writeFileSyncSpy).not.toHaveBeenCalled()

      readFileSyncSpy.mockReturnValue('older')
      start()
      expect(writeFileSyncSpy).toHaveBeenCalledWith(
        join(plugin.options.cssDir, 'devup-ui.css'),
        'base',
        'utf-8',
      )
    })

    it('compiles once more, writing no file, when a stylesheet changed', () => {
      getCssSpy.mockReturnValue('before')
      const { compiler, compilation, plugin, taps, run, start } = compile()
      start()
      servedCss(compilation).set('/df/devup-ui/devup-ui-1.css', 'before')
      getCssSpy.mockReturnValue('after')
      taps.finishModules!()
      taps.processAssets!()
      expect(compilation.deleteAsset).toHaveBeenCalledWith('main.js')
      expect(compilation.deleteAsset).toHaveBeenCalledWith('main.css')
      writeFileSyncSpy.mockClear()
      expect(taps.needAdditionalPass!()).toBe(true)
      expect(writeFileSyncSpy).toHaveBeenCalledWith(
        '/df/devup-ui/devup-ui-1.css',
        'after',
        'utf-8',
      )
      expect(writeFileSyncSpy).toHaveBeenCalledWith(
        join(plugin.options.cssDir, 'devup-ui.css'),
        'after',
        'utf-8',
      )

      // one more pass per run at most
      compilation.deleteAsset.mockClear()
      taps.processAssets!()
      expect(compilation.deleteAsset).not.toHaveBeenCalled()
      expect(taps.needAdditionalPass!()).toBeUndefined()
      run()
      expect(taps.needAdditionalPass!()).toBe(true)

      // a watching compiler rebuilds through the files the loaders write
      compiler.watchMode = true
      start()
      taps.finishModules!()
      expect(taps.needAdditionalPass!()).toBeUndefined()
    })

    it('keeps the pass when every stylesheet is current', () => {
      getCssSpy.mockReturnValue('same')
      const { compilation, taps, start } = compile()
      start()
      servedCss(compilation).set('/df/devup-ui/devup-ui-1.css', 'same')
      taps.finishModules!()
      taps.processAssets!()
      expect(compilation.deleteAsset).not.toHaveBeenCalled()
      expect(taps.needAdditionalPass!()).toBeUndefined()
    })
  })
})
