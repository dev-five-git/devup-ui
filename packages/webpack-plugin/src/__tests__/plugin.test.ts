import assert from 'node:assert/strict'
import type { Stats } from 'node:fs'
import * as fs from 'node:fs'
import * as fsPromises from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join, relative, resolve } from 'node:path'

import * as pluginUtils from '@devup-ui/plugin-utils'
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

const realExistsSync = fs.existsSync
const native = pluginUtils.loadCompiler('native')
const wasm = pluginUtils.loadCompiler('wasm')

describe('shorthand creation boundary', () => {
  it.each([undefined, 'native'] as const)(
    'registers real native shorthands when engine=%s',
    (engine) => {
      // Given
      native.resetStateForTesting()
      // When
      new DevupUIWebpackPlugin({
        engine,
        shorthands: { insetX: ['left', 'right'] },
      })
      const output = native.codeExtract(
        'engine.tsx',
        'import { Box } from "@devup-ui/react"; export const box = <Box insetX="19px" />',
        '@devup-ui/react',
        './df',
        true,
        false,
        true,
        {},
      )
      try {
        // Then
        expect(output.css).toContain('left:19px')
        expect(output.css).toContain('right:19px')
      } finally {
        output.free()
        native.resetStateForTesting()
      }
    },
  )

  it('rejects malformed shorthand before consulting the selector', () => {
    // Given
    const selection = spyOn(pluginUtils, 'loadCompiler')
    try {
      // When / Then
      expect(() =>
        Reflect.construct(DevupUIWebpackPlugin, [
          { engine: 'invalid', shorthands: { bgx: ['backgroundColour'] } },
        ]),
      ).toThrow(pluginUtils.ShorthandConfigError)
      expect(selection).not.toHaveBeenCalled()
    } finally {
      selection.mockRestore()
    }
  })

  it('registers canonical targets when the plugin is constructed', () => {
    const registration = spyOn(wasm, 'registerShorthands').mockReturnValue(
      undefined,
    )
    try {
      new DevupUIWebpackPlugin({
        engine: 'wasm',
        shorthands: { bgx: ['backgroundColor', 'py', '--Gap'] },
      })
      expect(registration.mock.calls).toEqual([
        [
          {
            bgx: ['background-color', 'padding-top', 'padding-bottom', '--Gap'],
          },
        ],
      ])
    } finally {
      registration.mockRestore()
    }
  })

  it('rejects a typo before registration when runtime options bypass types', () => {
    const registration = spyOn(wasm, 'registerShorthands').mockReturnValue(
      undefined,
    )
    try {
      expect(() =>
        Reflect.construct(DevupUIWebpackPlugin, [
          { shorthands: { bgx: ['backgroundColour'] } },
        ]),
      ).toThrow(pluginUtils.ShorthandConfigError)
      expect(registration).not.toHaveBeenCalled()
    } finally {
      registration.mockRestore()
    }
  })

  it('reuses canonical targets when the compiler initializes', () => {
    const registration = spyOn(wasm, 'registerShorthands').mockReturnValue(
      undefined,
    )
    const compiler = createCompiler()
    try {
      // Given
      const plugin = new DevupUIWebpackPlugin({
        engine: 'wasm',
        shorthands: { bgx: ['backgroundColor', 'py', '--Gap'] },
      })
      registration.mockClear()
      // When
      plugin.apply(asCompiler(compiler))
      // Then
      expect(registration.mock.calls).toEqual([
        [
          {
            bgx: ['background-color', 'padding-top', 'padding-bottom', '--Gap'],
          },
        ],
      ])
    } finally {
      for (const [, endBuild] of compiler.hooks.shutdown.tap.mock.calls)
        endBuild()
      registration.mockRestore()
    }
  })

  it('reads StyleX under the styleResolution it is given, again when the compiler initializes', () => {
    using resolution = spyOn(wasm, 'setStylexStyleResolution')
    const compiler = createCompiler()
    try {
      const plugin = new DevupUIWebpackPlugin({
        engine: 'wasm',
        stylex: { styleResolution: 'application-order' },
      })
      new DevupUIWebpackPlugin({ engine: 'wasm' })
      plugin.apply(asCompiler(compiler))
      expect(resolution.mock.calls).toEqual([
        ['application-order'],
        ['property-specificity'],
        ['application-order'],
      ])
    } finally {
      for (const [, endBuild] of compiler.hooks.shutdown.tap.mock.calls)
        endBuild()
      wasm.setStylexStyleResolution(null)
    }
  })

  it('rejects an unknown styleResolution before selecting a compiler', () => {
    using selection = spyOn(pluginUtils, 'loadCompiler')
    expect(() =>
      Reflect.construct(DevupUIWebpackPlugin, [
        { stylex: 'application-order' },
      ]),
    ).toThrow(pluginUtils.StylexConfigError)
    expect(selection).not.toHaveBeenCalled()
  })
})

type CodeExtractResult = pluginUtils.DevupCompilerOutput
interface MockCompiler {
  options: {
    module: { rules: unknown[] }
    plugins: unknown[]
    entry?: unknown
    context?: string
  }
  watchMode?: boolean
  webpack: {
    DefinePlugin: ReturnType<typeof mock>
    WebpackError: typeof Error
    Compilation: {
      PROCESS_ASSETS_STAGE_REPORT: number
      PROCESS_ASSETS_STAGE_OPTIMIZE?: number
    }
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
    dependencies: [],
    free: mock(),
    [Symbol.dispose]: mock(),
    ...overrides,
  }
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
let loadDevupConfigSyncSpy: ReturnType<typeof spyOn>
let existsSyncSpy: ReturnType<typeof spyOn>
let mkdirSyncSpy: ReturnType<typeof spyOn>
let readFileSyncSpy: ReturnType<typeof spyOn>
let writeFileSyncSpy: ReturnType<typeof spyOn>
let mkdirSpy: ReturnType<typeof spyOn>
let readFileSpy: ReturnType<typeof spyOn>
let statSpy: ReturnType<typeof spyOn>
let writeFileSpy: ReturnType<typeof spyOn>
let consoleErrorSpy: ReturnType<typeof spyOn>
let reservationSpy: ReturnType<typeof spyOn>

beforeEach(() => {
  reservationSpy = spyOn(
    pluginUtils,
    'createSharedSlotReservation',
  ).mockReturnValue({
    invalidate: mock(),
    reserve: mock().mockReturnValue(true),
  })
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
  consoleErrorSpy = spyOn(console, 'error').mockReturnValue(undefined)
})

afterEach(() => {
  reservationSpy.mockRestore()
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
  consoleErrorSpy.mockRestore()
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
      WebpackError: Error,
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
  it.each([
    ['complete', 'empty', false],
    ['empty', 'complete', false],
    ['complete', 'empty', true],
    ['empty', 'complete', true],
  ] as const)(
    'finalizes served production buckets after %s then %s with failed=%s',
    async (first, second, failed) => {
      // Given
      const compiler = createCompiler()
      const compilation = {}
      const plugin = new DevupUIWebpackPlugin({ engine: 'wasm' })
      plugin.apply(asCompiler(compiler))
      const bucket = join(plugin.options.cssDir, 'devup-ui-23.css')
      const other = join(plugin.options.cssDir, 'devup-ui-41.css')
      const unserved = join(plugin.options.cssDir, 'devup-ui-99.css')
      const complete = '.shared{color:red}'
      const otherCss = '.other{color:blue}'
      const disk = new Map<string, string>([
        [other, otherCss],
        [unserved, 'untouched'],
      ])
      for (const value of [first, second])
        disk.set(bucket, value === 'complete' ? complete : '')
      const before = [...disk]
      servedCss(compilation).set(bucket, complete)
      servedCss(compilation).set(other, otherCss)
      servedCss(compilation).set(
        join(plugin.options.cssDir, 'devup-ui.css'),
        'base',
      )
      getCssSpy.mockImplementation((fileNumber: number | null) =>
        fileNumber === 23 ? complete : fileNumber === 41 ? otherCss : 'base',
      )
      writeFileSpy.mockImplementation(
        (
          path: Parameters<typeof fsPromises.writeFile>[0],
          content: unknown,
        ) => {
          assert(typeof content === 'string')
          disk.set(String(path), content)
          return Promise.resolve()
        },
      )
      writeFileSpy.mockClear()
      try {
        // When
        await compiler.hooks.done.tapPromise.mock.calls[0][1]({
          compilation,
          hasErrors: () => failed,
        })
        // Then
        if (failed) {
          expect([...disk]).toEqual(before)
          expect(writeFileSpy).not.toHaveBeenCalled()
        } else {
          expect(Object.fromEntries(disk)).toEqual({
            [bucket]: complete,
            [other]: otherCss,
            [unserved]: 'untouched',
            [join(plugin.options.cssDir, 'devup-ui.css')]: 'base',
          })
        }
      } finally {
        for (const [, endBuild] of compiler.hooks.shutdown.tap.mock.calls)
          endBuild()
      }
    },
  )

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
        new DevupUIWebpackPlugin({
          engine: 'wasm',
          include: ['@acme/ui'],
          debug: true,
        }).apply(asCompiler(compiler))
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
        const warn = spyOn(console, 'warn').mockImplementation(() => {})
        try {
          new DevupUIWebpackPlugin({ engine: 'wasm', debug: true }).apply(
            asCompiler(createCompiler()),
          )
          expect(warn).toHaveBeenCalledWith(
            '[devup-ui] webpack file numbering failed: scan boom. Stylesheet file numbers follow the order webpack compiles files in, so they can differ between builds of the same source.',
          )
        } finally {
          warn.mockRestore()
        }
      } finally {
        collectSpy.mockRestore()
        seedSpy.mockRestore()
        resetSpy.mockRestore()
      }
    })
  })
  it.each([true, false])(
    'default regressions: should apply default options (omitted: %s)',
    (omitted) => {
      const plugin = omitted
        ? new DevupUIWebpackPlugin()
        : new DevupUIWebpackPlugin({})
      expect(plugin.options).toEqual({
        include: [],
        package: '@devup-ui/react',
        cssDir: resolve('df', 'devup-ui'),
        devupFile: 'devup.json',
        distDir: 'df',
        watch: false,
        debug: false,
        singleCss: false,
        engine: 'native',
        licenseBanner: true,
        atomHoist: 2,
      })
    },
  )

  describe('project root', () => {
    const context = resolve('apps', 'web')

    function applyAt(
      compilerContext: string | undefined,
      options: ConstructorParameters<typeof DevupUIWebpackPlugin>[0] = {},
    ) {
      const compiler = createCompiler()
      compiler.options.context = compilerContext
      const plugin = new DevupUIWebpackPlugin({ engine: 'wasm', ...options })
      plugin.apply(asCompiler(compiler))
      const [sourceRule] = compiler.options.module.rules as {
        use: { options: { root: string; sheetFile: string } }[]
      }[]
      compiler.hooks.shutdown.tap.mock.calls[0][1]()
      return { plugin, loader: sourceRule.use[0].options }
    }

    it("resolves the relative options against webpack's context", () => {
      const { plugin, loader } = applyAt(context, {
        include: ['lib'],
        productionPrewarmFiles: ['src/b.tsx', join(context, 'src', 'a.tsx')],
      })

      expect(plugin.options).toMatchObject({
        devupFile: join(context, 'devup.json'),
        distDir: join(context, 'df'),
        cssDir: join(context, 'df', 'devup-ui'),
        productionPrewarmFiles: ['src/a.tsx', 'src/b.tsx'],
      })
      expect(plugin.sheetFile).toBe(join(context, 'df', 'sheet.json'))
      expect(loader.root).toBe(context)
      expect(loader.sheetFile).toBe(join(context, 'df', 'sheet.json'))
      expect(mkdirSyncSpy).toHaveBeenCalledWith(join(context, 'df'), {
        recursive: true,
      })
      expect(loadDevupConfigSyncSpy).toHaveBeenCalledWith(
        join(context, 'devup.json'),
        expect.any(Set),
      )
    })

    it('resolves relative user paths against the context and keeps absolute ones', () => {
      const cssDir = resolve('elsewhere', 'css')
      const { plugin } = applyAt(context, {
        devupFile: 'theme/devup.json',
        distDir: 'out',
        cssDir,
      })

      expect(plugin.options).toMatchObject({
        devupFile: join(context, 'theme', 'devup.json'),
        distDir: join(context, 'out'),
        cssDir,
      })
    })

    it('keeps the paths as given when the context is the cwd', () => {
      const { plugin, loader } = applyAt(process.cwd(), {
        productionPrewarmFiles: ['src/a.tsx'],
      })

      expect(plugin.options).toMatchObject({
        devupFile: 'devup.json',
        distDir: 'df',
        cssDir: resolve('df', 'devup-ui'),
        productionPrewarmFiles: ['src/a.tsx'],
      })
      expect(plugin.sheetFile).toBe(join('df', 'sheet.json'))
      expect(loader.root).toBe(process.cwd())
    })

    it('falls back to the devup.json of the cwd with a warning', () => {
      const warn = spyOn(console, 'warn').mockReturnValue(undefined)
      existsSyncSpy.mockImplementation(
        (path: string) => path === resolve('devup.json'),
      )
      try {
        const fallbackContext = resolve('apps', 'fallback')
        const { plugin } = applyAt(fallbackContext)

        expect(plugin.options.devupFile).toBe(resolve('devup.json'))
        expect(plugin.options.distDir).toBe(join(fallbackContext, 'df'))
        expect(warn).toHaveBeenCalledTimes(1)
        expect(warn.mock.calls[0]?.[0]).toContain('devupFile')
      } finally {
        warn.mockRestore()
      }
    })

    it('forgets deleted sources under their names relative to the context', async () => {
      const compiler = createCompiler()
      compiler.options.context = context
      new DevupUIWebpackPlugin({ engine: 'wasm', watch: true }).apply(
        asCompiler(compiler),
      )
      using removeFile = spyOn(wasm, 'removeFile').mockReturnValue(false)
      Object.assign(compiler, {
        removedFiles: new Set([join(context, 'src', 'a.tsx')]),
      })
      await compiler.hooks.watchRun.tapPromise.mock.calls[0][1]()
      expect(removeFile.mock.calls).toEqual([['src/a.tsx']])
    })
  })

  it.each([false, true])(
    'forgets deleted sources at watchRun with watch=%s',
    async (watch) => {
      // Given: a compilation that deleted two sources, one with styles left
      const compiler = createCompiler()
      new DevupUIWebpackPlugin({ engine: 'wasm', watch }).apply(
        asCompiler(compiler),
      )
      using removeFile = spyOn(wasm, 'removeFile').mockImplementation(
        (file) => file === 'src/styled.tsx',
      )
      using exportSheet = spyOn(wasm, 'exportSheet').mockReturnValue('sheet')
      const watchRun = compiler.hooks.watchRun.tapPromise.mock.calls[0][1]
      // When: nothing was removed, then two files were
      await watchRun()
      Object.assign(compiler, {
        removedFiles: new Set([
          resolve('src', 'plain.tsx'),
          resolve('src', 'styled.tsx'),
        ]),
      })
      writeFileSyncSpy.mockClear()
      await watchRun()
      // Then: each is forgotten under its cwd-relative name, and only watch
      // persistence writes the pruned state
      expect(removeFile.mock.calls).toEqual([
        ['src/plain.tsx'],
        ['src/styled.tsx'],
      ])
      expect(
        writeFileSyncSpy.mock.calls.some(([, content]) => content === 'sheet'),
      ).toBe(watch)
      expect(exportSheet).toHaveBeenCalledTimes(watch ? 1 : 0)
    },
  )

  it('keeps the persisted state when no deleted source had styles', () => {
    const plugin = new DevupUIWebpackPlugin({ engine: 'wasm', watch: true })
    using removeFile = spyOn(wasm, 'removeFile').mockReturnValue(false)
    using exportSheet = spyOn(wasm, 'exportSheet').mockReturnValue('sheet')
    plugin.removeFiles(new Set([resolve('src', 'plain.tsx')]))
    expect(removeFile).toHaveBeenCalledTimes(1)
    expect(exportSheet).not.toHaveBeenCalled()
  })

  it('also reserves at native watchRun when watch persistence is disabled', async () => {
    const manager = {
      invalidate: mock(),
      reserve: mock().mockReturnValue(true),
    }
    reservationSpy.mockReturnValue(manager)
    const compiler = createCompiler()
    new DevupUIWebpackPlugin({ engine: 'wasm' }).apply(asCompiler(compiler))
    await compiler.hooks.watchRun.tapPromise.mock.calls[0][1]()
    expect(manager.reserve).toHaveBeenCalledTimes(1)
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
      expect(
        new DevupUIWebpackPlugin({ ...options, engine: 'wasm' }).options,
      ).toEqual({
        ...options,
        engine: 'wasm',
        licenseBanner: true,
        atomHoist: 2,
      })
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

      const plugin = new DevupUIWebpackPlugin({ ...options, engine: 'wasm' })
      await plugin.writeDataFiles()

      expect(loadDevupConfigSyncSpy).toHaveBeenCalledWith(
        options.devupFile,
        expect.any(Set),
      )
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
      // Written even when empty, so removed tokens leave no stale declarations.
      expect(writeFileSyncSpy).toHaveBeenCalledWith(
        join(options.distDir, 'theme.d.ts'),
        _options.getThemeInterface,
        {
          encoding: 'utf-8',
        },
      )

      if (options.watch)
        expect(writeFileSyncSpy).toHaveBeenCalledWith(
          join(options.cssDir, 'devup-ui.css'),
          _options.getCss,
        )
      expect(writeFileSyncSpy).toHaveBeenCalledTimes(options.watch ? 2 : 1)
    })
  })

  it.each(
    createTestMatrix({
      include: [
        {
          input: ['lib'],
          output: new RegExp(
            '((?:^|[\\/\\\\])node_modules[\\/\\\\](?!.*[\\/\\\\]node_modules[\\/\\\\])(?!(?:@devup-ui|@devup-editor|lib)(?:[\\/\\\\]|$)))|(.mdx.[tj]sx?$)',
          ),
        },
        {
          input: [],
          output: new RegExp(
            '((?:^|[\\/\\\\])node_modules[\\/\\\\](?!.*[\\/\\\\]node_modules[\\/\\\\])(?!(?:@devup-ui|@devup-editor)(?:[\\/\\\\]|$)))|(.mdx.[tj]sx?$)',
          ),
        },
        {
          input: ['lib', 'lib2'],
          output: new RegExp(
            '((?:^|[\\/\\\\])node_modules[\\/\\\\](?!.*[\\/\\\\]node_modules[\\/\\\\])(?!(?:@devup-ui|@devup-editor|lib|lib2)(?:[\\/\\\\]|$)))|(.mdx.[tj]sx?$)',
          ),
        },
      ],
    }),
  )('should set include', async (options) => {
    const plugin = new DevupUIWebpackPlugin({
      engine: 'wasm',
      include: options.include.input,
    })
    existsSyncSpy.mockReturnValue(false)

    const compiler = createCompiler()
    await plugin.apply(asCompiler(compiler))
    expect(compiler.options.module.rules.length).toBe(3)

    const sourceRule = compiler.options.module.rules[0]
    assert(
      typeof sourceRule === 'object' &&
        sourceRule !== null &&
        'exclude' in sourceRule,
    )
    expect(sourceRule.exclude).toEqual(options.include.output)
  })

  it('extracts MDX in a post loader, after the MDX loader compiled it', async () => {
    const plugin = new DevupUIWebpackPlugin({
      engine: 'wasm',
      include: ['lib'],
    })
    existsSyncSpy.mockReturnValue(false)

    const compiler = createCompiler()
    await plugin.apply(asCompiler(compiler))

    const [sourceRule, , mdxRule] = compiler.options.module.rules
    assert(typeof sourceRule === 'object' && sourceRule !== null)
    expect(mdxRule).toEqual({
      test: /\.mdx$/i,
      exclude: new RegExp(
        '(?:^|[\\/\\\\])node_modules[\\/\\\\](?!.*[\\/\\\\]node_modules[\\/\\\\])(?!(?:@devup-ui|@devup-editor|lib)(?:[\\/\\\\]|$))',
      ),
      enforce: 'post',
      use: 'use' in sourceRule ? sourceRule.use : undefined,
    })
  })

  it.each(
    createTestMatrix({
      debug: [true, false],
    }),
  )('should set debug', async (options) => {
    const plugin = new DevupUIWebpackPlugin({ ...options, engine: 'wasm' })

    const compiler = createCompiler()

    await plugin.apply(asCompiler(compiler))
    expect(setDebugSpy).toHaveBeenCalledWith(options.debug)
  })

  it('should reset data files when load error', async () => {
    const plugin = new DevupUIWebpackPlugin({
      engine: 'wasm',
      watch: true,
    })
    const compiler = createCompiler()
    readFileSyncSpy.mockImplementation(() => {
      throw new Error('error')
    })
    statSpy.mockResolvedValue(createStats(1))
    existsSyncSpy.mockReturnValue(true)
    plugin.apply(asCompiler(compiler))
    await compiler.hooks.watchRun.tapPromise.mock.calls[0][1]()
    expect(importSheetSpy).toHaveBeenCalledWith({})
    expect(importClassMapSpy).toHaveBeenCalledWith({})
    expect(importFileMapSpy).toHaveBeenCalledWith({})
  })

  it('watches the configs devup.json extends and reloads when one changes', async () => {
    const base = resolve('themes/base.json')
    loadDevupConfigSyncSpy.mockImplementation(
      (path: string, inputs?: Set<string>) => {
        inputs?.add(resolve(path))
        inputs?.add(base)
        return {}
      },
    )
    existsSyncSpy.mockImplementation(
      (path: string) => path === resolve('devup.json') || path === base,
    )
    const stamps = new Map([
      [resolve('devup.json'), 1],
      [base, 1],
    ])
    statSpy.mockImplementation(async (path: string) =>
      createStats(stamps.get(path) ?? 0),
    )
    const plugin = new DevupUIWebpackPlugin({ engine: 'wasm', watch: true })
    const compiler = createCompiler()
    plugin.apply(asCompiler(compiler))
    expect(plugin.configFiles).toEqual([resolve('devup.json'), base])

    const add = mock()
    compiler.hooks.afterCompile.tap.mock.calls[0][1]({
      fileDependencies: { add },
      missingDependencies: { add: mock() },
    })
    expect(add).toHaveBeenCalledWith(base)

    const watchRun = compiler.hooks.watchRun.tapPromise.mock.calls.find(
      ([name]) => name === 'DevupUIWebpackPlugin',
    )?.[1] as () => Promise<void>
    await watchRun()
    loadDevupConfigSyncSpy.mockClear()
    await watchRun()
    expect(loadDevupConfigSyncSpy).not.toHaveBeenCalled()

    // Editing the base theme reloads; so does deleting it
    stamps.set(base, 2)
    await watchRun()
    expect(loadDevupConfigSyncSpy).toHaveBeenCalledTimes(1)
    stamps.delete(base)
    await watchRun()
    expect(loadDevupConfigSyncSpy).toHaveBeenCalledTimes(2)
  })

  it('fails a one-shot build on a theme error', () => {
    const clash = new Error(
      "theme: color token 'card' and shadow token 'card' both become the CSS variable --card",
    )
    registerThemeSpy.mockImplementationOnce(() => {
      throw clash
    })
    const plugin = new DevupUIWebpackPlugin({ engine: 'wasm' })
    const compiler = createCompiler()

    expect(() => plugin.apply(asCompiler(compiler))).toThrow(clash)

    expect(registerThemeSpy).toHaveBeenCalledTimes(1)
    expect(consoleErrorSpy).not.toHaveBeenCalled()
    expect(compiler.options.module.rules).toEqual([])
  })

  it.each([
    ['an Error', new Error('bad token'), '[devup-ui] bad token'],
    ['a thrown string', 'bad token', '[devup-ui] bad token'],
  ])(
    'reports a theme error that is %s in every watch compilation until devup.json loads again',
    async (_name, thrown, message) => {
      registerThemeSpy.mockImplementationOnce(() => {
        throw thrown
      })
      existsSyncSpy.mockImplementation((path: string) => path === 'devup.json')
      statSpy.mockResolvedValueOnce(createStats(1))
      statSpy.mockResolvedValueOnce(createStats(2))
      const plugin = new DevupUIWebpackPlugin({ engine: 'wasm', watch: true })
      const compiler = createCompiler()

      plugin.apply(asCompiler(compiler))

      expect(consoleErrorSpy).toHaveBeenCalledWith(thrown)
      expect(registerThemeSpy).toHaveBeenLastCalledWith({})
      const setup = compiler.hooks.thisCompilation.tap.mock.calls.find(
        ([name]) => name === 'DevupUIWebpackPluginWatchCss',
      )?.[1] as (compilation: object) => void
      const compile = () => {
        const compilation = {
          errors: [] as Error[],
          hooks: { finishModules: { tapPromise: mock() } },
        }
        setup(compilation)
        return compilation.errors.map((error) => error.message)
      }
      expect(compile()).toEqual([message])
      expect(compile()).toEqual([message])

      const watchRun = compiler.hooks.watchRun.tapPromise.mock.calls.find(
        ([name]) => name === 'DevupUIWebpackPlugin',
      )?.[1] as () => Promise<void>
      await watchRun()
      await watchRun()

      expect(plugin.themeError).toBeUndefined()
      expect(compile()).toEqual([])
    },
  )

  describe('Tailwind CSS definitions', () => {
    const tailwindFile = resolve('styles/tailwind.css')

    function mockTailwindCss() {
      existsSyncSpy.mockImplementation(
        (path: string) =>
          path === 'devup.json' ||
          path === resolve('devup.json') ||
          path === tailwindFile,
      )
      loadDevupConfigSyncSpy.mockReturnValue({
        tailwind: { css: 'styles/tailwind.css' },
      })
      readFileSyncSpy.mockImplementation((path: string) =>
        path === tailwindFile ? '@theme { --color-brand: #0af; }' : '{}',
      )
    }

    it('registers the text of the Tailwind CSS and depends on its files', () => {
      mockTailwindCss()
      const plugin = new DevupUIWebpackPlugin({ engine: 'wasm' })
      const compiler = createCompiler()

      plugin.apply(asCompiler(compiler))

      expect(registerThemeSpy).toHaveBeenCalledWith({
        tailwindCss: '@theme { --color-brand: #0af; }',
      })
      const add = mock()
      compiler.hooks.afterCompile.tap.mock.calls[0][1]({
        fileDependencies: { add },
      })
      expect(add).toHaveBeenCalledWith(resolve('devup.json'))
      expect(add).toHaveBeenCalledWith(tailwindFile)
    })

    it('reads it again when a file changes', async () => {
      mockTailwindCss()
      const plugin = new DevupUIWebpackPlugin({ engine: 'wasm', watch: true })
      const compiler = createCompiler()
      statSpy.mockResolvedValueOnce(createStats(1))
      statSpy.mockResolvedValueOnce(createStats(1))
      statSpy.mockResolvedValueOnce(createStats(1))
      statSpy.mockResolvedValueOnce(createStats(2))
      plugin.apply(asCompiler(compiler))
      registerThemeSpy.mockClear()

      await compiler.hooks.watchRun.tapPromise.mock.calls[0][1]()
      await compiler.hooks.watchRun.tapPromise.mock.calls[0][1]()

      expect(registerThemeSpy).toHaveBeenCalledWith({
        tailwindCss: '@theme { --color-brand: #0af; }',
      })
    })

    it('takes a Tailwind CSS that is gone as unchanged', async () => {
      existsSyncSpy.mockImplementation((path: string) => path === tailwindFile)
      loadDevupConfigSyncSpy.mockReturnValue({
        tailwind: { css: 'styles/tailwind.css' },
      })
      readFileSyncSpy.mockImplementation((path: string) =>
        path === tailwindFile ? '@import "tailwindcss";' : '{}',
      )
      const plugin = new DevupUIWebpackPlugin({
        engine: 'wasm',
        watch: true,
        devupFile: 'missing.json',
      })
      const compiler = createCompiler()
      statSpy.mockRejectedValue(new Error('gone'))
      plugin.apply(asCompiler(compiler))

      await compiler.hooks.watchRun.tapPromise.mock.calls[0][1]()

      expect(plugin.tailwindFiles).toEqual([tailwindFile])
    })
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
      engine: 'wasm',
      watch: options.watch,
    })
    const compiler = createCompiler()

    existsSyncSpy.mockImplementation((path: string) => {
      if (path === resolve(plugin.options.devupFile))
        return options.existsDevupFile
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
      // A devup.json that does not exist yet is watched for being added
      expect(statSpy).toHaveBeenCalledWith(resolve(plugin.options.devupFile))
      await compiler.hooks.watchRun.tapPromise.mock.calls[0][1]()
    } else expect(compiler.hooks.watchRun.tapPromise).toHaveBeenCalledTimes(1)
    expect(compiler.hooks.afterCompile.tap).toHaveBeenCalled()
    const add = mock()
    const addMissing = mock()
    compiler.hooks.afterCompile.tap.mock.calls[0][1]({
      fileDependencies: { add },
      missingDependencies: { add: addMissing },
    })
    expect(options.existsDevupFile ? add : addMissing).toHaveBeenCalledWith(
      resolve(plugin.options.devupFile),
    )
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
        compilation: {},
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
    const plugin = new DevupUIWebpackPlugin({
      engine: 'wasm',
      prefix: 'my-prefix',
    })
    const compiler = createCompiler()
    existsSyncSpy.mockReturnValue(false)
    plugin.apply(asCompiler(compiler))
    expect(setPrefixSpy).toHaveBeenCalledWith('my-prefix')
  })

  it.each([
    [undefined, true],
    [true, true],
    [false, false],
  ] as const)(
    'sets the license banner from licenseBanner=%s',
    (licenseBanner, expected) => {
      using setLicenseBanner = spyOn(wasm, 'setLicenseBanner').mockReturnValue(
        undefined,
      )
      existsSyncSpy.mockReturnValue(false)
      new DevupUIWebpackPlugin({ engine: 'wasm', licenseBanner }).apply(
        asCompiler(createCompiler()),
      )
      expect(setLicenseBanner).toHaveBeenCalledWith(expected)
    },
  )

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

    it('plans collapse and hoisting over one graph of the src and app roots of the context', async () => {
      const project = fs.mkdtempSync(join(tmpdir(), 'devup-ui-webpack-roots-'))
      try {
        // The cwd's own `app` is not the project's: only the context's roots count
        for (const path of ['app/page.tsx', 'web/app/a.tsx', 'web/src/b.tsx'])
          await Bun.write(join(project, path), 'export {}')
        existsSyncSpy.mockImplementation((path) => realExistsSync(String(path)))
        const plugin = new DevupUIWebpackPlugin({
          engine: 'wasm',
          atomHoist: 2,
        })
        const compiler = createCompiler()
        compiler.options.context = join(project, 'web')
        const cwd = spyOn(process, 'cwd').mockReturnValue(project)
        try {
          plugin.apply(asCompiler(compiler))
        } finally {
          cwd.mockRestore()
        }
        const [{ graph }] = buildCanonicalMapSpy.mock.calls.at(-1) as [
          { graph: { files: string[] } },
        ]
        expect(graph.files).toEqual(
          ['web/app/a.tsx', 'web/src/b.tsx'].map((path) => join(project, path)),
        )
        expect(computeFileReachSpy).toHaveBeenCalledWith(
          expect.objectContaining({ graph, srcDir: join(project, 'web') }),
        )
      } finally {
        fs.rmSync(project, { recursive: true, force: true })
      }
    })

    it('runs single-importer collapse when atomHoist is explicitly zero, without hoisting', () => {
      // Constraint: single-importer collapse must ALWAYS be on. The canonical
      // map is built + imported unconditionally; only atom HOISTING is gated.
      buildCanonicalMapSpy.mockReturnValue({
        'src/child.tsx': 'src/parent.tsx',
      })
      const plugin = new DevupUIWebpackPlugin({ engine: 'wasm', atomHoist: 0 })
      plugin.apply(asCompiler(createCompiler()))
      expect(buildCanonicalMapSpy).toHaveBeenCalledWith(
        expect.objectContaining({ keyBy: 'cwd-relative' }),
      )
      expect(importCanonicalMapSpy).toHaveBeenCalled()
      // Explicit zero disables atoms, not always-on collapse.
      expect(computeFileReachSpy).not.toHaveBeenCalled()
      expect(setAtomHoistSpy).not.toHaveBeenCalled()
      expect(importFileRoutesSpy).not.toHaveBeenCalled()
    })

    describe('default regressions', () => {
      beforeEach(() => {
        computeFileReachSpy.mockReturnValue({
          'src/shared.tsx': [0, 1],
          'src/a.tsx': [0],
          'src/b.tsx': [1],
        })
      })

      it.each([
        ['omitted', {}],
        ['undefined', { atomHoist: undefined }],
      ])(
        'matches explicit 2 on two routes when atomHoist is %s',
        (_label, options) => {
          // Given: explicit 2 installs a known two-route plan.
          new DevupUIWebpackPlugin({ engine: 'wasm', atomHoist: 2 }).apply(
            asCompiler(createCompiler()),
          )
          const explicitReach = [...importFileRoutesSpy.mock.calls]
          expect(explicitReach).toEqual([
            [
              {
                'src/shared.tsx': [0, 1],
                'src/a.tsx': [0],
                'src/b.tsx': [1],
              },
            ],
          ])
          expect(setAtomHoistSpy).toHaveBeenCalledWith(2)
          importFileRoutesSpy.mockClear()
          setAtomHoistSpy.mockClear()
          // When: the public option is defaulted.
          new DevupUIWebpackPlugin({ ...options, engine: 'wasm' }).apply(
            asCompiler(createCompiler()),
          )
          // Then: the same plan and threshold are installed.
          expect(importFileRoutesSpy.mock.calls).toEqual(explicitReach)
          expect(setAtomHoistSpy.mock.calls).toEqual([[2]])
        },
      )

      it.each([0, -1, NaN, Infinity, -Infinity])(
        'keeps legacy OFF for atomHoist %s',
        (atomHoist) => {
          // Given: two routes, not the single-route fallback.
          // When: a disabled numeric option is supplied.
          new DevupUIWebpackPlugin({ engine: 'wasm', atomHoist }).apply(
            asCompiler(createCompiler()),
          )
          // Then: collapse still runs but reach and threshold are untouched.
          expect(buildCanonicalMapSpy).toHaveBeenCalled()
          expect(computeFileReachSpy).not.toHaveBeenCalled()
          expect(importFileRoutesSpy).not.toHaveBeenCalled()
          expect(setAtomHoistSpy).not.toHaveBeenCalled()
        },
      )

      it('preserves a custom threshold above the minimum', () => {
        // Given: two routes and a custom threshold.
        // When: the plugin applies.
        new DevupUIWebpackPlugin({ engine: 'wasm', atomHoist: 3 }).apply(
          asCompiler(createCompiler()),
        )
        // Then: the threshold is not replaced by 2.
        expect(setAtomHoistSpy.mock.calls).toEqual([[3]])
      })

      it.each([
        ['omitted', {}],
        ['undefined', { atomHoist: undefined }],
        ['enabled', { atomHoist: 2 }],
      ])(
        'ignores %s atomHoist before planning in actual singleCss mode',
        (_label, options) => {
          // Given: a two-entry compiler and prewarm inventory under explicit OFF.
          computeReachableFilesSpy.mockReturnValue([
            resolve('src/a.tsx'),
            resolve('src/b.tsx'),
          ])
          const run = (
            policy: ConstructorParameters<typeof DevupUIWebpackPlugin>[0],
          ) => {
            const compiler = createCompiler()
            compiler.options.entry = {
              a: { import: ['./src/a.tsx'] },
              b: { import: ['./src/b.tsx'] },
            }
            new DevupUIWebpackPlugin({
              ...policy,
              engine: 'wasm',
              singleCss: true,
            }).apply(asCompiler(compiler))
            compiler.hooks.run.tap.mock.calls[0][1]()
            return compiler.options.module.rules
          }
          const offRules = run({ atomHoist: 0 })
          const offExtractions = [...codeExtractSpy.mock.calls]
          expect(offExtractions).toHaveLength(2)
          codeExtractSpy.mockClear()
          // When: only the requested atom option changes.
          const onRules = run(options)
          // Then: no reach/threshold changes; loader and prewarm IDs/imports agree.
          expect(computeFileReachSpy).not.toHaveBeenCalled()
          expect(importFileRoutesSpy).not.toHaveBeenCalled()
          expect(setAtomHoistSpy).not.toHaveBeenCalled()
          expect(codeExtractSpy.mock.calls).toEqual(offExtractions)
          expect(codeExtractSpy).toHaveBeenCalledWith(
            'src/a.tsx',
            '{}',
            '@devup-ui/react',
            expect.any(String),
            true,
            false,
            true,
            expect.anything(),
          )
          expect(onRules).toEqual(offRules)
        },
      )
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
        resolve('/project', 'src', 'parent.tsx'),
        resolve('/project', 'src', 'child.tsx'),
      ])
      readFileSyncSpy.mockReturnValue('source')
      const setModuleResolverSpy = spyOn(
        wasm,
        'setModuleResolver',
      ).mockReturnValue(undefined)
      try {
        const plugin = new DevupUIWebpackPlugin({
          engine: 'wasm',
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
        compiler.hooks.run.tap.mock.calls[0][1]()
        expect(computeReachableFilesSpy).toHaveBeenCalledWith({
          srcDir: resolve('/project'),
          entries: [
            resolve('/project', './src/parent'),
            resolve('/project', './src/other'),
          ],
          graph: expect.objectContaining({ files: expect.any(Array) }),
        })
        // The loader's resolver is registered before the first extraction, so
        // imported constants inline as they do in the loader
        expect(setModuleResolverSpy).toHaveBeenCalledTimes(2)
        const firstExtraction = codeExtractSpy.mock.invocationCallOrder[0]
        assert(firstExtraction !== undefined)
        for (const order of setModuleResolverSpy.mock.invocationCallOrder)
          expect(order).toBeLessThan(firstExtraction)
        const resolvedModules: unknown[] = []
        for (const call of setModuleResolverSpy.mock.calls) {
          const resolveModule: unknown = call[0]
          assert(typeof resolveModule === 'function')
          const resolved: unknown = resolveModule(
            './plugin.test',
            import.meta.path,
          )
          expect(resolved).toMatchObject({
            path: relative(resolve('/project'), import.meta.path).replaceAll(
              '\\',
              '/',
            ),
          })
          resolvedModules.push(resolved)
        }
        expect(resolvedModules[1]).toEqual(resolvedModules[0])
        expect(codeExtractSpy).toHaveBeenCalledTimes(2)
        for (const result of codeExtractSpy.mock.results) {
          if (result.type === 'return')
            expect(result.value.free).toHaveBeenCalledTimes(1)
        }
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
      } finally {
        setModuleResolverSpy.mockRestore()
      }
    })

    it('uses normalized complete files instead of Next loader requests, including external files', () => {
      // Given an authoritative set with duplicate spellings and an external file.
      const seed = spyOn(wasm, 'seedFileMap').mockReturnValue(undefined)
      const shorthands = spyOn(wasm, 'registerShorthands').mockReturnValue(
        undefined,
      )
      const plugin = new DevupUIWebpackPlugin({
        engine: 'wasm',
        productionPrewarmFiles: [
          'src/z.tsx',
          resolve('node_modules/ui/a.tsx'),
          './src/z.tsx',
          'src\\z.tsx',
        ],
        shorthands: { insetX: ['left', 'right'] },
      })
      const compiler = createCompiler()
      compiler.options.entry = { main: { import: ['next-app-loader?query!'] } }
      try {
        // When the compiler applies and its registered preparation runs.
        plugin.apply(asCompiler(compiler))
        expect(codeExtractSpy).not.toHaveBeenCalled()
        compiler.hooks.run.tap.mock.calls[0][1]()
        // Then all files are extracted exactly once, in deterministic loader-ID order.
        expect(codeExtractSpy.mock.calls.map(([filename]) => filename)).toEqual(
          ['node_modules/ui/a.tsx', 'src/z.tsx'],
        )
        expect(computeReachableFilesSpy).not.toHaveBeenCalled()
        expect(seed).toHaveBeenCalledWith([
          'node_modules/ui/a.tsx',
          'src/z.tsx',
        ])
        expect(shorthands).toHaveBeenLastCalledWith({
          insetX: ['left', 'right'],
        })
      } finally {
        seed.mockRestore()
        shorthands.mockRestore()
      }
    })

    it("prewarms from the build's shared sources, read once by an earlier compiler", () => {
      // Given a source an earlier compiler of the build already read.
      const file = import.meta.path
      const sourceCache: pluginUtils.SourceCache = new Map()
      pluginUtils.withSourceCache(
        () => pluginUtils.readSource(file),
        sourceCache,
      )
      const [[key, entry]] = [...sourceCache]
      sourceCache.set(key, { ...entry, code: 'read before' })
      const plugin = new DevupUIWebpackPlugin({
        engine: 'wasm',
        productionPrewarmFiles: [file],
        sourceCache,
      })
      const compiler = createCompiler()
      // When this compiler prepares its compilation.
      plugin.apply(asCompiler(compiler))
      compiler.hooks.run.tap.mock.calls[0][1]()
      // Then it extracts what was read, without reading the file again.
      expect(codeExtractSpy.mock.calls.map(([, code]) => code)).toEqual([
        'read before',
      ])
      expect(globalThis.__devupSourceCache).toBeUndefined()
    })

    it.each([false, true])(
      'does not prewarm when the complete set is empty or watch=%s',
      (watch) => {
        // Given a reachable generic entry which an explicit set supersedes.
        computeReachableFilesSpy.mockReturnValue([resolve('src/parent.tsx')])
        const plugin = new DevupUIWebpackPlugin({
          engine: 'wasm',
          watch,
          productionPrewarmFiles: watch ? ['src/parent.tsx'] : [],
        })
        // When preparation runs, neither watch files nor generic fallback are extracted.
        const compiler = createCompiler()
        plugin.apply(asCompiler(compiler))
        compiler.hooks.run.tap.mock.calls[0][1]()
        expect(codeExtractSpy).not.toHaveBeenCalled()
        expect(computeReachableFilesSpy).not.toHaveBeenCalled()
      },
    )

    it.each(['canonical', 'numbering', 'read'])(
      'propagates genuine %s errors for an explicit complete set',
      (stage) => {
        // Given a failing correctness-prewarm dependency.
        const failure = new Error(stage)
        const endBuild = mock()
        const begin = spyOn(pluginUtils, 'beginBuild').mockReturnValue(endBuild)
        const fail = () => {
          throw failure
        }
        const numbering = spyOn(
          pluginUtils,
          'collectNumberedFiles',
        ).mockReturnValue([])
        if (stage === 'canonical') buildCanonicalMapSpy.mockImplementation(fail)
        if (stage === 'numbering') numbering.mockImplementation(fail)
        if (stage === 'read') readFileSyncSpy.mockImplementation(fail)
        const plugin = new DevupUIWebpackPlugin({
          engine: 'wasm',
          debug: stage === 'numbering',
          productionPrewarmFiles: ['src/parent.tsx'],
        })
        const compiler = createCompiler()
        try {
          // When its lifecycle stage runs, the original error reaches webpack.
          if (stage === 'canonical' || stage === 'numbering') {
            expect(() => plugin.apply(asCompiler(compiler))).toThrow(failure)
          } else {
            plugin.apply(asCompiler(compiler))
            expect(() => compiler.hooks.run.tap.mock.calls[0][1]()).toThrow(
              failure,
            )
            expect(endBuild).not.toHaveBeenCalled()
            compiler.hooks.shutdown.tap.mock.calls[0][1]()
          }
          expect(endBuild).toHaveBeenCalledTimes(1)
        } finally {
          begin.mockRestore()
          numbering.mockRestore()
        }
      },
    )

    it('skips a file of an explicit complete set it cannot extract, with a warning', () => {
      // Given a complete set holding a dead file that does not parse.
      readFileSyncSpy.mockImplementation((path: string) => String(path))
      codeExtractSpy.mockImplementation(
        (filename: string, contents: string) => {
          if (filename === 'src/dead.tsx')
            throw new Error('Cannot parse source: Unexpected token')
          return createCodeExtractResult(contents)
        },
      )
      const plugin = new DevupUIWebpackPlugin({
        engine: 'wasm',
        productionPrewarmFiles: ['src/a.tsx', 'src/dead.tsx', 'src/z.tsx'],
      })
      const compiler = createCompiler()
      const warn = spyOn(console, 'warn').mockImplementation(() => {})
      try {
        plugin.apply(asCompiler(compiler))
        // When the warm-up runs, the build goes on past the dead file.
        expect(() => compiler.hooks.run.tap.mock.calls[0][1]()).not.toThrow()
        // Then every file is still tried, and the dead one is named.
        expect(codeExtractSpy.mock.calls.map(([filename]) => filename)).toEqual(
          ['src/a.tsx', 'src/dead.tsx', 'src/z.tsx'],
        )
        expect(warn).toHaveBeenCalledWith(
          '[devup-ui] webpack stylesheet warm-up of src/dead.tsx failed: Cannot parse source: Unexpected token. The file is skipped; if webpack compiles it, the build fails with this error.',
        )
      } finally {
        warn.mockRestore()
      }
    })

    it('pre-warms without collapse, from no entries when they are a function', () => {
      buildCanonicalMapSpy.mockReturnValue({})
      const compiler = createCompiler()
      compiler.options.entry = () => ({})
      new DevupUIWebpackPlugin({ engine: 'wasm' }).apply(asCompiler(compiler))
      compiler.hooks.run.tap.mock.calls[0][1]()
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
      const plugin = new DevupUIWebpackPlugin({ engine: 'wasm', watch: true })
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
      const plugin = new DevupUIWebpackPlugin({ engine: 'wasm' })
      const warn = spyOn(console, 'warn').mockImplementation(() => {})
      try {
        // apply must still complete without throwing
        const compiler = createCompiler()
        plugin.apply(asCompiler(compiler))
        compiler.hooks.run.tap.mock.calls[0][1]()
        expect(codeExtractSpy).toHaveBeenCalled()
        expect(warn).toHaveBeenCalledWith(
          '[devup-ui] webpack stylesheet warm-up of src/parent.tsx failed: extract boom. The file is skipped; if webpack compiles it, the build fails with this error.',
        )
      } finally {
        warn.mockRestore()
      }
    })

    it('stops a generic pre-warm it cannot read, with a warning', () => {
      const parent = resolve(process.cwd(), 'src', 'parent.tsx')
      computeReachableFilesSpy.mockReturnValue([parent])
      readFileSyncSpy.mockImplementation((path: string) => {
        if (path === parent) throw new Error('read boom')
        return '{}'
      })
      const plugin = new DevupUIWebpackPlugin({ engine: 'wasm' })
      const warn = spyOn(console, 'warn').mockImplementation(() => {})
      try {
        const compiler = createCompiler()
        plugin.apply(asCompiler(compiler))
        compiler.hooks.run.tap.mock.calls[0][1]()
        expect(codeExtractSpy).not.toHaveBeenCalled()
        expect(warn).toHaveBeenCalledWith(
          '[devup-ui] webpack stylesheet warm-up failed: read boom. A stylesheet webpack builds before every file it holds is extracted can miss their styles.',
        )
      } finally {
        warn.mockRestore()
      }
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
      const plugin = new DevupUIWebpackPlugin({ engine: 'wasm', atomHoist: 2 })
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
      const plugin = new DevupUIWebpackPlugin({ engine: 'wasm', atomHoist: 1 })
      plugin.apply(asCompiler(createCompiler()))
      expect(setAtomHoistSpy).toHaveBeenCalledWith(2)
    })

    it('stays off when fewer than two routes are reachable', () => {
      computeFileReachSpy.mockReturnValue({ 'src/a.tsx': [0] })
      const plugin = new DevupUIWebpackPlugin({ engine: 'wasm', atomHoist: 2 })
      plugin.apply(asCompiler(createCompiler()))
      expect(setAtomHoistSpy).not.toHaveBeenCalled()
      expect(importFileRoutesSpy).not.toHaveBeenCalled()
    })

    it('swallows pre-pass errors (atom hoisting stays off)', () => {
      buildCanonicalMapSpy.mockImplementation(() => {
        throw new Error('boom')
      })
      const plugin = new DevupUIWebpackPlugin({ engine: 'wasm', atomHoist: 2 })
      const warn = spyOn(console, 'warn').mockImplementation(() => {})
      try {
        // apply must still complete without throwing
        plugin.apply(asCompiler(createCompiler()))
        expect(setAtomHoistSpy).not.toHaveBeenCalled()
        expect(warn).toHaveBeenCalledWith(
          '[devup-ui] webpack import graph failed: boom. Classes are not merged across files only one module imports, and atom hoisting is off.',
        )
      } finally {
        warn.mockRestore()
      }
    })

    it('warns without atom hoisting when a pre-pass outside atom mode fails', () => {
      buildCanonicalMapSpy.mockImplementation(() => {
        throw new Error('boom')
      })
      const warn = spyOn(console, 'warn').mockImplementation(() => {})
      try {
        new DevupUIWebpackPlugin({ engine: 'wasm', atomHoist: 0 }).apply(
          asCompiler(createCompiler()),
        )
        expect(warn).toHaveBeenCalledWith(
          '[devup-ui] webpack import graph failed: boom. Classes are not merged across files only one module imports.',
        )
      } finally {
        warn.mockRestore()
      }
    })

    it('stays quiet when the project has no src directory to scan', () => {
      buildCanonicalMapSpy.mockImplementation(() => {
        throw Object.assign(new Error('ENOENT'), {
          code: 'ENOENT',
          path: resolve(process.cwd(), 'src'),
        })
      })
      const warn = spyOn(console, 'warn').mockImplementation(() => {})
      try {
        new DevupUIWebpackPlugin({ engine: 'wasm', atomHoist: 2 }).apply(
          asCompiler(createCompiler()),
        )
        expect(warn).not.toHaveBeenCalled()
      } finally {
        warn.mockRestore()
      }
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
      const plugin = new DevupUIWebpackPlugin({ engine: 'wasm', atomHoist: 2 })
      plugin.apply(asCompiler(compiler))
      // pre-pass must run before module rules are pushed (single WASM instance)
      expect(rulesLenAtSetAtomHoist).toBe(0)
      expect(compiler.options.module.rules.length).toBeGreaterThan(0)
    })
  })

  describe('another pass for stylesheets built too early', () => {
    function compile(watch = false) {
      const compiler = createCompiler()
      const plugin = new DevupUIWebpackPlugin({ engine: 'wasm', watch })
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
      const run = compiler.hooks.run.tap.mock.calls.find(
        ([name]) => name === 'DevupUIWebpackPlugin',
      )?.[1] as () => void
      const start = () => {
        ;(
          compiler.hooks.thisCompilation.tap.mock.calls[0]![1] as (
            compilation: object,
          ) => void
        )(compilation)
      }
      return { compiler, plugin, compilation, taps, run, start }
    }

    it('uses same-compilation reconciliation instead of another pass in watch mode', async () => {
      const { compiler } = compile(true)
      expect(compiler.hooks.run.tap.mock.calls.map(([name]) => name)).toEqual([
        'DevupUIWebpackPluginReservation',
      ])
      expect(
        compiler.hooks.thisCompilation.tap.mock.calls.map(([name]) => name),
      ).toEqual(['DevupUIWebpackPluginWatchCss'])
      type Module = { readonly resource: string }
      type Finish = (modules: Iterable<Module>) => Promise<void>
      const completions: Finish[] = []
      const rebuild = mock((_module: Module, done: () => void) => done())
      const compilation = {
        hooks: {
          finishModules: {
            tapPromise(
              _options: { readonly name: string; readonly stage: number },
              callback: Finish,
            ) {
              completions.push(callback)
            },
          },
        },
        rebuildModule: rebuild,
      }
      const setup = compiler.hooks.thisCompilation.tap.mock.calls[0]?.[1]
      assert(typeof setup === 'function')
      setup(compilation)
      const [finish] = completions
      assert(finish, 'watch setup must install reconciliation completion')
      const module = { resource: '/df/devup-ui/devup-ui-1.css' }
      servedCss(compilation).set(module.resource, 'before')
      getCssSpy.mockClear()
      getCssSpy.mockReturnValue('after')
      await finish([module])
      expect(getCssSpy).toHaveBeenCalledWith(1, true)
      expect(rebuild).toHaveBeenCalledWith(module, expect.any(Function))
    })

    it('drops unselected component markers only in a one-shot production build', () => {
      const { compiler } = compile()
      compiler.options.mode = 'production'
      const run = compiler.hooks.run.tap.mock.calls.find(
        ([name]) => name === 'DevupUIWebpackPluginReservation',
      )![1] as () => void
      compiler.webpack.Compilation.PROCESS_ASSETS_STAGE_OPTIMIZE = 700
      const ledger = pluginUtils.markerLedgerOf(compiler)
      assert(ledger)
      const scanned = spyOn(ledger, 'scanned')
      run()
      expect(scanned).toHaveBeenCalledTimes(1)
      const plan = spyOn(ledger, 'plan')
      const stages: Record<string, [number, () => void]> = {}
      const compilation = {
        modules: [],
        warnings: [],
        getAssets: () => [],
        hooks: {
          finishModules: { tap: mock() },
          needAdditionalPass: { tap: mock() },
          processAssets: {
            tap: (
              { name, stage }: { name: string; stage: number },
              fn: () => void,
            ) => {
              stages[name] = [stage, fn]
            },
          },
        },
      }
      ;(
        compiler.hooks.thisCompilation.tap.mock.calls[0]![1] as (
          compilation: object,
        ) => void
      )(compilation)
      const [stage, strip] = stages['DevupUIWebpackPluginComponentMarkers']!
      expect(stage).toBe(700)
      compiler.watchMode = true
      strip()
      expect(plan).not.toHaveBeenCalled()
      compiler.watchMode = false
      strip()
      expect(plan).toHaveBeenCalledTimes(1)
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
