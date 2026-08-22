import * as fs from 'node:fs'
import * as fsPromises from 'node:fs/promises'
import * as nodePath from 'node:path'

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

import { DevupUI } from '../plugin'

type CodeExtractResult = ReturnType<typeof wasm.codeExtract>
interface ConfigHookMeta {
  viteVersion?: string
  rollupVersion?: string
  rolldownVersion?: string
}
interface CodeSplittingGroup {
  name: (id: string) => string | null
  minSize?: number
  minShareCount?: number
}
interface ViteOutputOptions {
  manualChunks?: (id: string, code: string) => string | undefined
  codeSplitting?: { groups?: CodeSplittingGroup[] }
}
interface ViteConfig {
  build?: {
    rollupOptions?: { output?: ViteOutputOptions }
    rolldownOptions?: { output?: ViteOutputOptions }
  }
  optimizeDeps?: { exclude?: string[] }
  ssr?: { noExternal?: RegExp[] }
  define?: Record<string, string>
}

interface ViteTestPlugin {
  name: string
  enforce: 'pre'
  apply: () => boolean
  config: (
    this: { meta?: ConfigHookMeta } | void,
    userConfig?: ViteConfig,
  ) => ViteConfig
  configResolved: (config?: {
    command?: 'serve' | 'build'
    root?: string
  }) => Promise<void>
  watchChange: (id: string) => Promise<void>
  handleHotUpdate: (context: {
    file: string
    server: {
      moduleGraph: {
        invalidateModule: (...args: unknown[]) => void
      }
      ws: { send: (...args: unknown[]) => void }
    }
    modules: object[]
    timestamp: number
  }) => Promise<unknown[] | undefined>
  load: (id: string) => string | undefined
  transform: (code: string, id: string) => Promise<{ code: string } | undefined>
  generateBundle: (
    options: object,
    bundle: Record<string, { source: string; name: string }>,
  ) => Promise<void>
  resolveId: (source: string, importer?: string) => string | undefined
}

function createCodeExtractResult(
  overrides: Partial<CodeExtractResult> = {},
): CodeExtractResult {
  return {
    css: 'css code',
    code: 'code',
    cssFile: 'devup-ui.css',
    map: undefined,
    updatedBaseStyle: false,
    free: mock(),
    [Symbol.dispose]: mock(),
    ...overrides,
  } as unknown as CodeExtractResult
}

function createPlugin(options?: Parameters<typeof DevupUI>[0]): ViteTestPlugin {
  return DevupUI(options) as unknown as ViteTestPlugin
}

const ROLLUP_META: ConfigHookMeta = {
  viteVersion: '7.1.14',
  rollupVersion: '4.52.5',
}
const ROLLDOWN_META: ConfigHookMeta = {
  viteVersion: '8.2.2',
  rollupVersion: '4.52.5',
  rolldownVersion: '1.2.5',
}
const ROLLDOWN_VITE_META: ConfigHookMeta = {
  viteVersion: '7.1.14',
  rollupVersion: '4.52.5',
  rolldownVersion: '1.2.5',
}

function callConfig(
  plugin: ViteTestPlugin,
  meta?: ConfigHookMeta,
  userConfig?: ViteConfig,
): ViteConfig {
  return plugin.config.call(meta ? { meta } : undefined, userConfig)
}

const { basename, join, resolve, relative: originalRelative } = nodePath

let existsSyncSpy: ReturnType<typeof spyOn>
let mkdirSpy: ReturnType<typeof spyOn>
let readFileSpy: ReturnType<typeof spyOn>
let writeFileSpy: ReturnType<typeof spyOn>
let relativeSpy: ReturnType<typeof spyOn>
let codeExtractSpy: ReturnType<typeof spyOn>
let getCssSpy: ReturnType<typeof spyOn>
let getDefaultThemeSpy: ReturnType<typeof spyOn>
let getThemeInterfaceSpy: ReturnType<typeof spyOn>
let registerThemeSpy: ReturnType<typeof spyOn>
let setDebugSpy: ReturnType<typeof spyOn>
let setPrefixSpy: ReturnType<typeof spyOn>

beforeEach(() => {
  existsSyncSpy = spyOn(fs, 'existsSync').mockReturnValue(false)
  mkdirSpy = spyOn(fsPromises, 'mkdir').mockResolvedValue(undefined)
  readFileSpy = spyOn(fsPromises, 'readFile').mockResolvedValue('{}')
  writeFileSpy = spyOn(fsPromises, 'writeFile').mockResolvedValue(undefined)
  relativeSpy = spyOn(nodePath, 'relative').mockImplementation(
    (from: string, to: string) => originalRelative(from, to),
  )
  codeExtractSpy = spyOn(wasm, 'codeExtract').mockReturnValue(
    createCodeExtractResult(),
  )
  getCssSpy = spyOn(wasm, 'getCss').mockReturnValue('css code')
  getDefaultThemeSpy = spyOn(wasm, 'getDefaultTheme').mockReturnValue('default')
  getThemeInterfaceSpy = spyOn(wasm, 'getThemeInterface').mockReturnValue(
    'interface code',
  )
  registerThemeSpy = spyOn(wasm, 'registerTheme').mockReturnValue(undefined)
  setDebugSpy = spyOn(wasm, 'setDebug').mockReturnValue(undefined)
  setPrefixSpy = spyOn(wasm, 'setPrefix').mockReturnValue(undefined)
})

afterEach(() => {
  existsSyncSpy.mockRestore()
  mkdirSpy.mockRestore()
  readFileSpy.mockRestore()
  writeFileSpy.mockRestore()
  relativeSpy.mockRestore()
  codeExtractSpy.mockRestore()
  getCssSpy.mockRestore()
  getDefaultThemeSpy.mockRestore()
  getThemeInterfaceSpy.mockRestore()
  registerThemeSpy.mockRestore()
  setDebugSpy.mockRestore()
  setPrefixSpy.mockRestore()
})

describe('devupUIVitePlugin', () => {
  console.error = mock()

  it('should apply default options', () => {
    const plugin = createPlugin({})
    expect(plugin).toEqual({
      name: 'devup-ui',
      config: expect.any(Function),
      load: expect.any(Function),
      watchChange: expect.any(Function),
      handleHotUpdate: expect.any(Function),
      enforce: 'pre',
      transform: expect.any(Function),
      apply: expect.any(Function),
      generateBundle: expect.any(Function),
      configResolved: expect.any(Function),
      resolveId: expect.any(Function),
    })
    expect(plugin.apply()).toBe(true)
  })

  it.each(
    globalThis.createTestMatrix({
      debug: [true, false],
      extractCss: [true, false],
    }),
  )('should apply options', async (options) => {
    const plugin = createPlugin(options)
    expect(setDebugSpy).toHaveBeenCalledWith(options.debug)
    if (options.extractCss) {
      expect(
        callConfig(
          plugin,
          ROLLUP_META,
        ).build?.rollupOptions?.output?.manualChunks?.('devup-ui.css', 'code'),
      ).toEqual('devup-ui.css')

      expect(
        callConfig(
          plugin,
          ROLLUP_META,
        ).build?.rollupOptions?.output?.manualChunks?.('other.css', 'code'),
      ).toEqual(undefined)

      expect(
        callConfig(
          plugin,
          ROLLDOWN_META,
        ).build?.rolldownOptions?.output?.codeSplitting?.groups?.[0]?.name(
          'devup-ui.css',
        ),
      ).toEqual('devup-ui.css')

      expect(
        callConfig(
          plugin,
          ROLLDOWN_META,
        ).build?.rolldownOptions?.output?.codeSplitting?.groups?.[0]?.name(
          'other.css',
        ),
      ).toBeNull()
    } else {
      expect(callConfig(plugin, ROLLUP_META).build).toBeUndefined()
      expect(callConfig(plugin, ROLLDOWN_META).build).toBeUndefined()
    }
  })

  describe('devup css chunk merging', () => {
    const devupCssIds = [
      'devup-ui.css',
      'devup-ui-0.css',
      'devup-ui-12.css',
      join('/p', 'df', 'devup-ui', 'devup-ui-3.css'),
      `${join('/p', 'df', 'devup-ui', 'devup-ui.css')}?t=1730000000000`,
    ]
    const otherIds = [
      'other.css',
      'devup-ui.js',
      'my-devup-ui-styles.css',
      join('/p', 'src', 'app.tsx'),
    ]

    function rolldownGroup(meta: ConfigHookMeta) {
      const build = callConfig(createPlugin({}), meta).build
      const groups =
        build?.rolldownOptions?.output?.codeSplitting?.groups ??
        build?.rollupOptions?.output?.codeSplitting?.groups
      expect(groups).toHaveLength(1)
      return groups![0]!
    }

    it('names every devup css module after its own file on rollup', () => {
      const manualChunks = callConfig(createPlugin({}), ROLLUP_META).build
        ?.rollupOptions?.output?.manualChunks
      for (const id of devupCssIds) {
        expect(manualChunks?.(id, 'code')).toEqual(basename(id).split('?')[0])
      }
      for (const id of otherIds) {
        expect(manualChunks?.(id, 'code')).toBeUndefined()
      }
    })

    it('names every devup css module after its own file on rolldown', () => {
      const group = rolldownGroup(ROLLDOWN_META)
      for (const id of devupCssIds) {
        expect(group.name(id)).toEqual(basename(id).split('?')[0])
      }
      for (const id of otherIds) {
        expect(group.name(id)).toBeNull()
      }
    })

    it('opts the group out of a framework minSize / minShareCount fallback', () => {
      // Framework plugins set `codeSplitting.minSize` (vinext uses 10_000 for
      // its client environment); without an explicit per-group value that
      // fallback folds the devup css chunks back into every route chunk.
      const group = rolldownGroup(ROLLDOWN_META)
      expect(group.minSize).toBe(0)
      expect(group.minShareCount).toBe(1)
    })

    const chunkingCases: [
      string,
      ConfigHookMeta | undefined,
      'manualChunks' | 'codeSplitting',
    ][] = [
      ['rollup', ROLLUP_META, 'manualChunks'],
      ['rolldown on vite 8', ROLLDOWN_META, 'codeSplitting'],
      ['rolldown on vite 7', ROLLDOWN_VITE_META, 'codeSplitting'],
      ['unknown bundler', undefined, 'manualChunks'],
      [
        'rolldown without a vite version',
        { rolldownVersion: '1.2.5' },
        'codeSplitting',
      ],
    ]

    it.each(chunkingCases)(
      'picks the %s chunking option',
      (_name, meta, expected) => {
        const build = callConfig(createPlugin({}), meta).build
        // Vite drops one of the two when a plugin returns both.
        expect(build?.rollupOptions && build?.rolldownOptions).toBeFalsy()
        const output =
          build?.rolldownOptions?.output ?? build?.rollupOptions?.output
        expect(output?.manualChunks !== undefined).toBe(
          expected === 'manualChunks',
        )
        expect(output?.codeSplitting !== undefined).toBe(
          expected === 'codeSplitting',
        )
      },
    )

    const optionKeyCases: [string, ConfigHookMeta, boolean][] = [
      ['vite 8', ROLLDOWN_META, true],
      ['rolldown-vite on vite 7', ROLLDOWN_VITE_META, false],
    ]

    it.each(optionKeyCases)(
      'puts rolldown options under the right key on %s',
      (_name, meta, usesRolldownOptions) => {
        const build = callConfig(createPlugin({}), meta).build
        expect(build?.rolldownOptions !== undefined).toBe(usesRolldownOptions)
        expect(build?.rollupOptions !== undefined).toBe(!usesRolldownOptions)
      },
    )

    it('chains the user manualChunks instead of replacing it', () => {
      const userManualChunks = mock((id: string) =>
        id === 'vendor.js' ? 'vendor' : undefined,
      )
      const output = callConfig(createPlugin({}), ROLLUP_META, {
        build: {
          rollupOptions: { output: { manualChunks: userManualChunks } },
        },
      }).build?.rollupOptions?.output

      expect(output?.manualChunks?.('devup-ui.css', 'code')).toEqual(
        'devup-ui.css',
      )
      expect(output?.manualChunks?.('vendor.js', 'code')).toEqual('vendor')
      expect(output?.manualChunks?.('other.js', 'code')).toBeUndefined()
      expect(userManualChunks).not.toHaveBeenCalledWith('devup-ui.css', 'code')
    })

    it('reads the user manualChunks from the array form of output', () => {
      const userManualChunks = mock(() => 'vendor')
      const output = callConfig(createPlugin({}), ROLLUP_META, {
        build: {
          rollupOptions: {
            output: [{ manualChunks: userManualChunks }] as never,
          },
        },
      }).build?.rollupOptions?.output

      expect(output?.manualChunks?.('devup-ui.css', 'code')).toEqual(
        'devup-ui.css',
      )
      expect(output?.manualChunks?.('vendor.js', 'code')).toEqual('vendor')
    })

    it.each([
      ['no user output', {} as ViteConfig],
      [
        'a non-function manualChunks',
        {
          build: {
            rollupOptions: { output: { manualChunks: { a: ['b'] } as never } },
          },
        } as ViteConfig,
      ],
    ])('tolerates %s', (_name, userConfig) => {
      const output = callConfig(createPlugin({}), ROLLUP_META, userConfig).build
        ?.rollupOptions?.output
      expect(output?.manualChunks?.('devup-ui.css', 'code')).toEqual(
        'devup-ui.css',
      )
      expect(output?.manualChunks?.('other.js', 'code')).toBeUndefined()
    })
  })

  describe('deterministic css output', () => {
    it('creates the css dir before writing into it', async () => {
      const order: string[] = []
      existsSyncSpy.mockReturnValue(false)
      mkdirSpy.mockImplementation(async (dir: string) => {
        order.push(`mkdir:${dir}`)
        await new Promise((r) => setTimeout(r, 5))
        order.push(`mkdir-done:${dir}`)
        return undefined
      })
      writeFileSpy.mockImplementation(async (file: string) => {
        order.push(`write:${file}`)
        return undefined
      })

      await createPlugin({}).configResolved()

      const cssDir = resolve('df', 'devup-ui')
      const mkdirDone = order.indexOf(`mkdir-done:${cssDir}`)
      const wroteCss = order.indexOf(`write:${join(cssDir, 'devup-ui.css')}`)
      expect(mkdirDone).toBeGreaterThanOrEqual(0)
      expect(wroteCss).toBeGreaterThan(mkdirDone)
    })

    it('rewrites every devup css asset from the finished sheet', async () => {
      getCssSpy.mockImplementation(
        (fileNum: number | null) => `sheet:${fileNum}`,
      )
      const plugin = createPlugin({})
      const bundle = {
        'base.css': { source: 'stale', name: 'devup-ui.css' },
        'three.css': { source: 'stale', name: 'devup-ui-3.css' },
        'other.css': { source: 'keep', name: 'other.css' },
        'chunk.js': { name: 'devup-ui-9.css' },
      } as unknown as Record<string, { source: string; name: string }>

      await plugin.generateBundle({}, bundle)

      expect(bundle['base.css'].source).toEqual('sheet:null')
      expect(bundle['three.css'].source).toEqual('sheet:3')
      expect(bundle['other.css'].source).toEqual('keep')
      expect(bundle['chunk.js']).not.toHaveProperty('source')
    })

    it('ignores the load-time snapshot so output does not depend on module order', async () => {
      getCssSpy.mockReturnValue('early partial sheet')
      const plugin = createPlugin({})
      plugin.load('devup-ui.css')

      getCssSpy.mockReturnValue('final complete sheet')
      const bundle = { 'base.css': { source: '', name: 'devup-ui.css' } }
      await plugin.generateBundle({}, bundle)

      expect(bundle['base.css'].source).toEqual('final complete sheet')
    })

    it('resolves a stable id during build', async () => {
      const plugin = createPlugin({})
      await plugin.configResolved({ command: 'build' })
      const importer = join('df', 'devup-ui', 'devup-ui.css')

      const first = plugin.resolveId('devup-ui.css', importer)
      const second = plugin.resolveId('devup-ui.css', importer)

      expect(first).toEqual(join(resolve('df', 'devup-ui'), 'devup-ui.css'))
      expect(first).toEqual(second)
      expect(first).not.toContain('?t=')
    })

    it('cache-busts the id during dev so a growing sheet invalidates', async () => {
      const plugin = createPlugin({})
      await plugin.configResolved({ command: 'serve' })

      expect(
        plugin.resolveId(
          'devup-ui.css',
          join('df', 'devup-ui', 'devup-ui.css'),
        ),
      ).toContain('?t=')
    })
  })

  it('should include default editor packages in vite config', () => {
    const plugin = createPlugin({})
    const config = callConfig(plugin)

    expect(config.optimizeDeps!.exclude).toEqual([
      '@devup-ui/components',
      '@devup-editor/react',
    ])
    expect(config.ssr!.noExternal).toEqual([/@devup-ui/, /@devup-editor/])
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
      getDefaultTheme: ['theme', ''],
      singleCss: [true, false],
    }),
  )('should write data files', async (options) => {
    writeFileSpy.mockResolvedValueOnce(undefined)
    readFileSpy.mockResolvedValueOnce(JSON.stringify({}))
    getThemeInterfaceSpy.mockReturnValue('interface code')
    getDefaultThemeSpy.mockReturnValue(options.getDefaultTheme)
    existsSyncSpy.mockImplementation((path: string) => {
      if (path === 'devup.json') return options.existsDevupFile
      if (path === 'df') return options.existsDistDir
      if (path === resolve('df', 'devup-ui')) return options.existsCssDir
      if (path === join('df', 'sheet.json')) return options.existsSheetFile
      if (path === join('df', 'classMap.json'))
        return options.existsClassMapFile
      if (path === join('df', 'fileMap.json')) return options.existsFileMapFile
      return false
    })
    const plugin = createPlugin({ singleCss: options.singleCss })
    await plugin.configResolved()
    if (options.existsDevupFile) {
      expect(readFileSpy).toHaveBeenCalledWith('devup.json', 'utf-8')
      expect(registerThemeSpy).toHaveBeenCalledWith({})
      expect(getThemeInterfaceSpy).toHaveBeenCalledWith(
        '@devup-ui/react',
        'CustomColors',
        'DevupThemeTypography',
        'CustomLength',
        'CustomShadows',
        'DevupTheme',
      )
      expect(writeFileSpy).toHaveBeenCalledWith(
        join('df', 'theme.d.ts'),
        'interface code',
        'utf-8',
      )
    } else {
      expect(registerThemeSpy).toHaveBeenCalledWith({})
    }

    const config = callConfig(plugin)
    if (options.getDefaultTheme) {
      expect(config.define).toEqual({
        'process.env.DEVUP_UI_DEFAULT_THEME': JSON.stringify(
          options.getDefaultTheme,
        ),
      })
    } else {
      expect(config.define).toEqual({})
    }
  })

  it('should reset data files when load error', async () => {
    writeFileSpy.mockResolvedValueOnce(undefined)
    getThemeInterfaceSpy.mockReturnValue('interface code')
    existsSyncSpy.mockReturnValue(true)
    readFileSpy.mockImplementation(() => {
      throw new Error('error')
    })
    const plugin = createPlugin({})
    await plugin.configResolved()
    expect(registerThemeSpy).toHaveBeenCalledWith({})
    expect(writeFileSpy).toHaveBeenCalledWith(
      join('df', '.gitignore'),
      '*',
      'utf-8',
    )
  })

  it('should watch change', async () => {
    writeFileSpy.mockResolvedValueOnce(undefined)
    getThemeInterfaceSpy.mockReturnValue('interface code')
    existsSyncSpy.mockReturnValue(true)
    readFileSpy.mockResolvedValueOnce(JSON.stringify({ theme: 'theme' }))
    const plugin = createPlugin({})
    await plugin.watchChange('devup.json')
    expect(writeFileSpy).toHaveBeenCalledWith(
      join('df', 'theme.d.ts'),
      'interface code',
      'utf-8',
    )

    await plugin.watchChange('wrong')
  })

  it('should invalidate and reload on devup hot update', async () => {
    writeFileSpy.mockResolvedValueOnce(undefined)
    getThemeInterfaceSpy.mockReturnValue('interface code')
    existsSyncSpy.mockReturnValue(true)
    readFileSpy.mockResolvedValueOnce(JSON.stringify({ theme: 'theme' }))
    const invalidateModule = mock()
    const send = mock()
    const module = {}
    const plugin = createPlugin({})

    const result = await plugin.handleHotUpdate({
      file: 'devup.json',
      server: {
        moduleGraph: { invalidateModule },
        ws: { send },
      },
      modules: [module],
      timestamp: 1,
    })

    expect(writeFileSpy).toHaveBeenCalledWith(
      join('df', 'theme.d.ts'),
      'interface code',
      'utf-8',
    )
    expect(invalidateModule).toHaveBeenCalledWith(
      module,
      expect.any(Set),
      1,
      true,
    )
    expect(send).toHaveBeenCalledWith({ type: 'full-reload' })
    expect(result).toEqual([])
  })

  it('should skip hot update for unrelated files', async () => {
    existsSyncSpy.mockReturnValue(true)
    const invalidateModule = mock()
    const send = mock()
    const plugin = createPlugin({})

    const result = await plugin.handleHotUpdate({
      file: 'other.json',
      server: {
        moduleGraph: { invalidateModule },
        ws: { send },
      },
      modules: [],
      timestamp: 1,
    })

    expect(result).toBeUndefined()
    expect(writeFileSpy).not.toHaveBeenCalledWith(
      join('df', 'theme.d.ts'),
      expect.any(String),
      'utf-8',
    )
    expect(invalidateModule).not.toHaveBeenCalled()
    expect(send).not.toHaveBeenCalled()
  })

  it('should print error when watch change error', async () => {
    writeFileSpy.mockResolvedValueOnce(undefined)
    getThemeInterfaceSpy.mockReturnValue('interface code')
    existsSyncSpy.mockReturnValueOnce(true).mockReturnValueOnce(false)
    mkdirSpy.mockImplementation(() => {
      throw new Error('error')
    })
    const plugin = createPlugin({})
    await plugin.watchChange('devup.json')
    expect(console.error).toHaveBeenCalledWith(expect.any(Error))
  })

  it('should load', () => {
    getCssSpy.mockReturnValue('css code')
    const plugin = createPlugin({})
    expect(plugin.load('devup-ui.css')).toEqual(expect.any(String))
    expect(plugin.load('devup-ui-10.css')).toEqual(expect.any(String))
  })

  it.each(
    createTestMatrix({
      extractCss: [true, false],
      updatedBaseStyle: [true, false],
    }),
  )('should transform', async (options) => {
    getCssSpy.mockReturnValue('css code')
    codeExtractSpy.mockReturnValue(
      createCodeExtractResult({
        css: 'css code',
        code: 'code',
        cssFile: 'devup-ui.css',
        map: undefined,
        updatedBaseStyle: options.updatedBaseStyle,
      }),
    )

    const plugin = createPlugin(options)

    expect(await plugin.transform('code', 'devup-ui.wrong')).toEqual(undefined)
    expect(await plugin.transform('code', 'devup-ui.tsx')).toEqual(
      options.extractCss ? { code: 'code' } : undefined,
    )

    if (options.extractCss) {
      expect(
        await plugin.transform('code', 'node_modules/test/index.tsx'),
      ).toEqual(undefined)
      expect(
        await plugin.transform(
          'code',
          'node_modules/@devup-ui/hello/index.tsx',
        ),
      ).toEqual({ code: 'code' })
      expect(
        await plugin.transform(
          'code',
          'node_modules/@devup-editor/react/index.tsx',
        ),
      ).toEqual({ code: 'code' })

      codeExtractSpy.mockReturnValue(
        createCodeExtractResult({
          css: 'css code test next',
          code: 'code',
          cssFile: 'devup-ui.css',
          map: undefined,
          updatedBaseStyle: options.updatedBaseStyle,
        }),
      )
      expect(writeFileSpy).toHaveBeenCalledWith(
        join(resolve('df', 'devup-ui'), 'devup-ui.css'),
        expect.stringMatching(
          /\/\* node_modules[/\\]@devup-ui[/\\]hello[/\\]index\.tsx \d+ \*\//,
        ),
        'utf-8',
      )
      expect(
        await plugin.transform(
          'code',
          'node_modules/@devup-ui/hello/index.tsx',
        ),
      ).toEqual({ code: 'code' })
    }
    expect(await plugin.load('devup-ui.css')).toEqual(expect.any(String))

    codeExtractSpy.mockReturnValue(
      createCodeExtractResult({
        css: 'long css code',
        code: 'long code',
        cssFile: 'devup-ui.css',
        map: undefined,
        updatedBaseStyle: options.updatedBaseStyle,
      }),
    )
    expect(await plugin.transform('code', 'devup-ui.tsx')).toEqual(
      options.extractCss ? { code: 'long code' } : undefined,
    )
  })

  it.each(
    createTestMatrix({
      extractCss: [true, false],
    }),
  )('should generateBundle', async (options) => {
    getCssSpy.mockReturnValue('css code test')
    const plugin = createPlugin({
      extractCss: options.extractCss,
      singleCss: true,
    })
    const bundle: Record<string, { source: string; name: string }> = {
      'devup-ui.css': { source: 'css code', name: 'devup-ui.css' },
    }
    plugin.load('devup-ui.css')
    await plugin.generateBundle({}, bundle)
    if (options.extractCss) {
      expect(bundle['devup-ui.css'].source).toEqual('css code test')
    } else {
      expect(bundle['devup-ui.css'].source).toEqual('css code')
    }
  })

  it('should resolveId', () => {
    getCssSpy.mockReturnValue('css code')
    {
      const plugin = createPlugin({})
      expect(
        plugin.resolveId('devup-ui.css', 'df/devup-ui/devup-ui.css'),
      ).toEqual(expect.any(String))

      expect(plugin.resolveId('other.css', 'df/devup-ui/devup-ui.css')).toEqual(
        undefined,
      )
    }

    {
      const plugin = createPlugin({
        cssDir: '',
      })
      expect(plugin.resolveId('devup-ui.css')).toEqual(expect.any(String))
    }
  })

  it('should resolve id with cssMap', () => {
    getCssSpy.mockReturnValue('css code')
    const plugin = createPlugin({})
    expect(plugin.load('devup-ui.css')).toEqual(expect.any(String))
    expect(plugin.load('other.css')).toEqual(undefined)

    expect(
      plugin.resolveId('devup-ui.css', 'df/devup-ui/devup-ui.css'),
    ).toEqual(expect.any(String))
  })

  it('should not write interface code when no theme', async () => {
    readFileSpy.mockResolvedValueOnce(JSON.stringify({}))
    getThemeInterfaceSpy.mockReturnValue('')
    existsSyncSpy.mockReturnValue(true)
    const plugin = createPlugin({})
    await plugin.configResolved()
    expect(writeFileSpy).not.toHaveBeenCalledWith(
      join('df', 'theme.d.ts'),
      expect.any(String),
      'utf-8',
    )
  })

  it('sholud add relative path to css file', async () => {
    getCssSpy.mockReturnValue('css code')
    codeExtractSpy.mockReturnValue(
      createCodeExtractResult({
        css: 'css code',
        code: 'code',
        cssFile: 'devup-ui.css',
        map: undefined,
        updatedBaseStyle: false,
      }),
    )
    const plugin = createPlugin({})
    relativeSpy.mockReturnValue('./df/devup-ui/devup-ui.css')
    await plugin.transform('code', 'foo.tsx')

    expect(codeExtractSpy).toHaveBeenCalledWith(
      'foo.tsx',
      'code',
      '@devup-ui/react',
      './df/devup-ui/devup-ui.css',
      false,
      true,
      false,
      {
        '@emotion/styled': 'styled',
        '@vanilla-extract/css': null,
        'styled-components': 'styled',
      },
    )

    relativeSpy.mockReturnValue('df/devup-ui/devup-ui.css')
    await plugin.transform('code', 'foo.tsx')
    expect(codeExtractSpy).toHaveBeenCalledWith(
      'foo.tsx',
      'code',
      '@devup-ui/react',
      './df/devup-ui/devup-ui.css',
      false,
      true,
      false,
      {
        '@emotion/styled': 'styled',
        '@vanilla-extract/css': null,
        'styled-components': 'styled',
      },
    )
  })

  it('should not create css file when cssFile is empty', async () => {
    getCssSpy.mockReturnValue('css code')
    codeExtractSpy.mockReturnValue(
      createCodeExtractResult({
        css: 'css code',
        code: 'code',
        cssFile: '',
        map: undefined,
        updatedBaseStyle: false,
      }),
    )
    const plugin = createPlugin({})
    await plugin.transform('code', 'foo.tsx')
    expect(writeFileSpy).not.toHaveBeenCalled()
  })

  it('should not generate bundle when css file is not found', async () => {
    const plugin = createPlugin({})
    const bundle = {}
    await plugin.generateBundle({}, bundle)
    expect(bundle).toEqual({})
  })

  it('should call setPrefix when prefix option is provided', () => {
    DevupUI({ prefix: 'my-prefix' })
    expect(setPrefixSpy).toHaveBeenCalledWith('my-prefix')
  })
})

describe('devupUIVitePlugin atom hoisting', () => {
  type ConfigResolved = (config: unknown) => Promise<void>
  const runConfigResolved = async (
    options: Parameters<typeof DevupUI>[0],
    config: unknown,
  ) => {
    const plugin = DevupUI(options) as unknown as {
      configResolved: ConfigResolved
    }
    await plugin.configResolved(config)
  }

  let buildCanonicalMapSpy: ReturnType<typeof spyOn>
  let computeFileReachSpy: ReturnType<typeof spyOn>
  let importCanonicalMapSpy: ReturnType<typeof spyOn>
  let importFileRoutesSpy: ReturnType<typeof spyOn>
  let setAtomHoistSpy: ReturnType<typeof spyOn>

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
  })

  afterEach(() => {
    buildCanonicalMapSpy.mockRestore()
    computeFileReachSpy.mockRestore()
    importCanonicalMapSpy.mockRestore()
    importFileRoutesSpy.mockRestore()
    setAtomHoistSpy.mockRestore()
  })

  it('does nothing when atomHoist is unset', async () => {
    await runConfigResolved({}, { root: '/p' })
    expect(buildCanonicalMapSpy).not.toHaveBeenCalled()
    expect(setAtomHoistSpy).not.toHaveBeenCalled()
  })

  it('composes collapse + hoist and folds reach onto the canonical bucket', async () => {
    buildCanonicalMapSpy.mockReturnValue({
      '/p/src/child.tsx': '/p/src/parent.tsx',
      '/p/src/glob.tsx': '@global',
    })
    computeFileReachSpy.mockReturnValue({
      '/p/src/parent.tsx': [0, 1],
      '/p/src/child.tsx': [0],
      '/p/src/glob.tsx': [0, 1],
      '/p/src/r1.tsx': [1],
    })
    await runConfigResolved(
      { atomHoist: 2 },
      { root: '/p', build: { rollupOptions: { input: { a: 'src/a.tsx' } } } },
    )
    // collapse runs (composition) with absolute keys
    expect(buildCanonicalMapSpy).toHaveBeenCalledWith(
      expect.objectContaining({ keyBy: 'absolute' }),
    )
    expect(importCanonicalMapSpy).toHaveBeenCalled()
    // reach folded by bucket: child -> parent, @global skipped
    expect(importFileRoutesSpy).toHaveBeenCalledWith({
      '/p/src/parent.tsx': [0, 1],
      '/p/src/r1.tsx': [1],
    })
    expect(setAtomHoistSpy).toHaveBeenCalledWith(2)
  })

  it('clamps the threshold to a minimum of 2', async () => {
    computeFileReachSpy.mockReturnValue({
      '/p/src/a.tsx': [0],
      '/p/src/b.tsx': [1],
    })
    await runConfigResolved({ atomHoist: 1 }, { root: '/p' })
    expect(setAtomHoistSpy).toHaveBeenCalledWith(2)
  })

  it('stays off when fewer than two routes are reachable', async () => {
    computeFileReachSpy.mockReturnValue({ '/p/src/a.tsx': [0] })
    await runConfigResolved({ atomHoist: 2 }, { root: '/p' })
    expect(setAtomHoistSpy).not.toHaveBeenCalled()
  })

  it('falls back to the heuristic when input has no JS entries', async () => {
    computeFileReachSpy.mockReturnValue({
      '/p/src/a.tsx': [0],
      '/p/src/b.tsx': [1],
    })
    await runConfigResolved(
      { atomHoist: 2 },
      { root: '/p', build: { rollupOptions: { input: 'index.html' } } },
    )
    // html-only input => entries override omitted => computeFileReach called
    // without an explicit entries list
    expect(computeFileReachSpy).toHaveBeenCalledWith(
      expect.objectContaining({ entries: undefined }),
    )
    expect(setAtomHoistSpy).toHaveBeenCalledWith(2)
  })

  it('accepts array and string JS entries', async () => {
    computeFileReachSpy.mockReturnValue({
      '/p/src/a.tsx': [0],
      '/p/src/b.tsx': [1],
    })
    await runConfigResolved(
      { atomHoist: 2 },
      { root: '/p', build: { rollupOptions: { input: ['src/a.tsx'] } } },
    )
    await runConfigResolved(
      { atomHoist: 2 },
      { root: '/p', build: { rollupOptions: { input: 'src/a.tsx' } } },
    )
    expect(setAtomHoistSpy).toHaveBeenCalledWith(2)
  })

  it('swallows pre-pass errors (atom hoisting stays off)', async () => {
    buildCanonicalMapSpy.mockImplementation(() => {
      throw new Error('boom')
    })
    await runConfigResolved({ atomHoist: 2 }, { root: '/p' })
    expect(setAtomHoistSpy).not.toHaveBeenCalled()
  })

  it('uses process.cwd() when config.root is absent', async () => {
    computeFileReachSpy.mockReturnValue({
      '/p/src/a.tsx': [0],
      '/p/src/b.tsx': [1],
    })
    await runConfigResolved({ atomHoist: 2 }, {})
    expect(setAtomHoistSpy).toHaveBeenCalledWith(2)
  })
})
