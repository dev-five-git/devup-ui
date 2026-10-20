import * as fs from 'node:fs'
import * as fsPromises from 'node:fs/promises'
import * as os from 'node:os'
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

describe('shorthand creation boundary', () => {
  it('registers canonical targets when the plugin is created', () => {
    const registration = spyOn(wasm, 'registerShorthands').mockReturnValue(
      undefined,
    )
    try {
      DevupUI({ shorthands: { bgx: ['backgroundColor', 'py', '--Gap'] } })
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
        Reflect.apply(DevupUI, undefined, [
          { shorthands: { bgx: ['backgroundColour'] } },
        ]),
      ).toThrow(pluginUtils.ShorthandConfigError)
      expect(registration).not.toHaveBeenCalled()
    } finally {
      registration.mockRestore()
    }
  })
})

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

interface HotUpdateEnvironment {
  config: { consumer: 'client' | 'server' }
  moduleGraph: { invalidateModule: (...args: unknown[]) => void }
  hot: { send: (...args: unknown[]) => void }
}

interface ViteTestPlugin {
  environment?: {
    name: string
    config: {
      consumer: 'client' | 'server'
      build?: { write?: boolean }
    }
  }
  name: string
  sharedDuringBuild: true
  enforce: 'pre'
  apply: () => boolean
  closeBundle: () => void
  config: (
    this: { meta?: ConfigHookMeta } | void,
    userConfig?: ViteConfig,
  ) => ViteConfig
  configResolved: (config?: {
    command?: 'serve' | 'build'
    root?: string
    plugins?: readonly object[]
  }) => Promise<void>
  watchChange: (id: string) => Promise<void>
  hotUpdate: (
    this: { environment: HotUpdateEnvironment },
    options: { file: string; modules: object[]; timestamp: number },
  ) => Promise<unknown[] | undefined>
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
  transform: (
    this: {
      environment?: {
        name: string
        config: {
          consumer: 'client' | 'server'
          build?: { write?: boolean }
        }
      }
    },
    code: string,
    id: string,
  ) => Promise<{ code: string } | undefined>
  generateBundle: (
    this: {
      environment?: {
        name: string
        config: {
          consumer: 'client' | 'server'
          build?: { write?: boolean }
        }
      }
    },
    options: object,
    bundle: Record<
      string,
      {
        source?: string
        name: string
        viteMetadata?: { importedCss?: Set<string> }
      }
    >,
  ) => Promise<void>
  resolveId: (source: string, importer?: string) => string | undefined
}

interface ViteTestRestorePlugin {
  name: string
  sharedDuringBuild: true
  apply: 'build'
  generateBundle: {
    order: 'post'
    handler: (options: object, bundle: Record<string, object>) => void
  }
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

function createPlugins(
  options?: Parameters<typeof DevupUI>[0],
): [ViteTestPlugin, ViteTestRestorePlugin] {
  return DevupUI(options) as unknown as [ViteTestPlugin, ViteTestRestorePlugin]
}

function createPlugin(options?: Parameters<typeof DevupUI>[0]): ViteTestPlugin {
  return createPlugins(options)[0]
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
let consoleErrorSpy: ReturnType<typeof spyOn>

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
  consoleErrorSpy = spyOn(console, 'error').mockReturnValue(undefined)
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
  consoleErrorSpy.mockRestore()
})

describe('devupUIVitePlugin', () => {
  it('should apply default options', () => {
    const plugin = createPlugin({})
    expect(plugin).toEqual({
      name: 'devup-ui',
      sharedDuringBuild: true,
      closeBundle: expect.any(Function),
      config: expect.any(Function),
      load: expect.any(Function),
      watchChange: expect.any(Function),
      handleHotUpdate: expect.any(Function),
      hotUpdate: expect.any(Function),
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
      'my-devup-ui.css',
      'vendor-devup-ui.css',
      'vendor-devup-ui-3.css',
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

  describe('deterministic file numbering', () => {
    const collectNumberedFiles = pluginUtils.collectNumberedFiles
    const seedFileMap = wasm.seedFileMap
    let collectSpy: ReturnType<typeof spyOn>
    let seedFileMapSpy: ReturnType<typeof spyOn>

    beforeEach(() => {
      collectSpy = spyOn(pluginUtils, 'collectNumberedFiles')
      seedFileMapSpy = spyOn(wasm, 'seedFileMap').mockReturnValue(undefined)
    })

    afterEach(() => {
      collectSpy.mockRestore()
      seedFileMapSpy.mockRestore()
    })

    function onlyDirs(...dirs: string[]) {
      const wanted = new Set(dirs.map((d) => resolve('/p', d)))
      existsSyncSpy.mockImplementation((path: string) => wanted.has(path))
    }

    it('numbers the files the scan finds, in the order it returns them', async () => {
      onlyDirs('src', 'app')
      collectSpy.mockReturnValue(['/p/app/page.tsx', '/p/src/b.tsx'])

      await createPlugin({ include: ['@acme/ui'] }).configResolved({
        root: '/p',
      })

      expect(collectSpy).toHaveBeenCalledWith({
        roots: [resolve('/p', 'src'), resolve('/p', 'app')],
        include: ['@acme/ui'],
        cwd: '/p',
        importAliases: {
          '@emotion/react': null,
          '@emotion/styled': 'styled',
          '@emotion/css': null,
          'styled-components': 'styled',
          '@vanilla-extract/css': null,
        },
        needles: expect.arrayContaining([
          '@devup-ui/react',
          '@stylexjs/stylex',
        ]),
      })
      expect(seedFileMapSpy).toHaveBeenCalledWith([
        '/p/app/page.tsx',
        '/p/src/b.tsx',
      ])
    })

    // A framework plugin resolves the config once per environment. Seeding
    // keeps the numbers files hold, so a second pass numbers only new files.
    it('seeds again on a second configResolved', async () => {
      onlyDirs('src')
      collectSpy.mockReturnValue(['/p/src/a.tsx'])
      const plugin = createPlugin({})

      await plugin.configResolved({ root: '/p' })
      await plugin.configResolved({ root: '/p' })

      expect(seedFileMapSpy).toHaveBeenCalledTimes(2)
    })

    it('leaves numbering alone when there is nothing to number', async () => {
      onlyDirs()
      collectSpy.mockReturnValue([])
      await createPlugin({}).configResolved({ root: '/p' })
      expect(seedFileMapSpy).not.toHaveBeenCalled()
    })

    it('keeps building when the scan fails', async () => {
      onlyDirs('src')
      collectSpy.mockImplementation(() => {
        throw new Error('scan boom')
      })

      await createPlugin({}).configResolved({ root: '/p' })

      expect(seedFileMapSpy).not.toHaveBeenCalled()
    })

    describe('source collection before seeding', () => {
      let listSourceFilesSpy: ReturnType<typeof spyOn>
      let readFileSyncSpy: ReturnType<typeof spyOn>

      beforeEach(() => {
        collectSpy.mockImplementation(collectNumberedFiles)
        listSourceFilesSpy = spyOn(pluginUtils, 'listSourceFiles')
        readFileSyncSpy = spyOn(fs, 'readFileSync').mockReturnValue(
          "import { Box } from '@devup-ui/react'",
        )
      })

      afterEach(() => {
        listSourceFilesSpy.mockRestore()
        readFileSyncSpy.mockRestore()
      })

      it('numbers files by sorted path, not by transform arrival order', async () => {
        onlyDirs('src')
        listSourceFilesSpy.mockReturnValue([
          '/p/src/z.tsx',
          '/p/src/a.tsx',
          '/p/src/m.tsx',
        ])

        await createPlugin({}).configResolved({ root: '/p' })

        expect(seedFileMapSpy).toHaveBeenCalledWith([
          '/p/src/a.tsx',
          '/p/src/m.tsx',
          '/p/src/z.tsx',
        ])
      })

      it('normalizes windows separators to match vite module ids', async () => {
        onlyDirs('src')
        listSourceFilesSpy.mockReturnValue(['C:\\p\\src\\a.tsx'])

        await createPlugin({}).configResolved({ root: '/p' })

        expect(seedFileMapSpy).toHaveBeenCalledWith(['C:/p/src/a.tsx'])
      })

      it('leaves an already-populated map alone on a second configResolved', async () => {
        onlyDirs('src')
        listSourceFilesSpy.mockReturnValue(['/p/src/a.tsx'])
        const previousMap = wasm.exportFileMap()
        const existingMap = {
          '/p/src/a.tsx': 0,
          '/monorepo/packages/ui/X.tsx': 1,
        }
        seedFileMapSpy.mockImplementation(seedFileMap)
        const plugin = createPlugin({})
        wasm.importFileMap(existingMap)
        try {
          await plugin.configResolved({ root: '/p' })
          await plugin.configResolved({ root: '/p' })

          expect(seedFileMapSpy).toHaveBeenCalledTimes(2)
          expect(JSON.parse(wasm.exportFileMap())).toEqual(existingMap)
        } finally {
          wasm.importFileMap(JSON.parse(previousMap))
        }
      })

      it('scans app/ for App Router projects and dedupes across roots', async () => {
        onlyDirs('src', 'app')
        listSourceFilesSpy.mockImplementation((dir: string) =>
          dir === resolve('/p', 'app')
            ? ['/p/app/page.tsx', '/p/shared.tsx']
            : ['/p/src/b.tsx', '/p/shared.tsx'],
        )

        await createPlugin({}).configResolved({ root: '/p' })

        expect(listSourceFilesSpy).toHaveBeenCalledWith(resolve('/p', 'src'))
        expect(listSourceFilesSpy).toHaveBeenCalledWith(resolve('/p', 'app'))
        expect(seedFileMapSpy).toHaveBeenCalledWith([
          '/p/app/page.tsx',
          '/p/shared.tsx',
          '/p/src/b.tsx',
        ])
      })

      it.each([
        ['no conventional source dir exists', () => onlyDirs()],
        [
          'the source dir is empty',
          () => {
            onlyDirs('src')
            listSourceFilesSpy.mockReturnValue([])
          },
        ],
      ])('leaves numbering alone when %s', async (_name, setup) => {
        setup()

        await createPlugin({}).configResolved({ root: '/p' })

        expect(seedFileMapSpy).not.toHaveBeenCalled()
      })
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
        'nested.css': { source: 'stale', name: 'assets/devup-ui-4.css' },
        'other.css': { source: 'keep', name: 'other.css' },
        'chunk.js': { name: 'devup-ui-9.css' },
      } as unknown as Record<string, { source: string; name: string }>

      await plugin.generateBundle({}, bundle)

      expect(bundle['base.css'].source).toEqual('sheet:null')
      expect(bundle['three.css'].source).toEqual('sheet:3')
      expect(bundle['nested.css'].source).toEqual('sheet:4')
      expect(bundle['other.css'].source).toEqual('keep')
      expect(bundle['chunk.js']).not.toHaveProperty('source')
    })

    it('leaves an app asset whose name merely ends in devup-ui.css alone', async () => {
      getCssSpy.mockImplementation(
        (fileNum: number | null) => `sheet:${fileNum}`,
      )
      const bundle = {
        'vendor.css': { source: 'app styles', name: 'vendor-devup-ui.css' },
      } as unknown as Record<string, { source: string; name: string }>

      await createPlugin({}).generateBundle({}, bundle)

      expect(bundle['vendor.css'].source).toEqual('app styles')
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

    describe('css forwarded from the server bundle by @vitejs/plugin-rsc', () => {
      interface Asset {
        type: 'asset'
        fileName: string
        source: string
        name: string
      }
      interface Chunk {
        type: 'chunk'
        fileName: string
        name: string
        viteMetadata?: { importedCss?: Set<string> }
      }
      type Output = Asset | Chunk
      type Bundle = Record<string, Output>

      const asset = (fileName: string, name: string): Asset => ({
        type: 'asset',
        fileName,
        source: 'stale',
        name,
      })
      const chunk = (fileName: string, css: string[]): Chunk => ({
        type: 'chunk',
        fileName,
        name: fileName,
        viteMetadata: { importedCss: new Set(css) },
      })
      const serverEnv = {
        environment: { name: 'rsc', config: { consumer: 'server' as const } },
      }
      const clientEnv = {
        environment: {
          name: 'client',
          config: { consumer: 'client' as const },
        },
      }
      const rscConfig = (bundles: Record<string, Bundle>) => ({
        plugins: [{ name: 'rsc:minimal', api: { manager: { bundles } } }],
      })

      /**
       * What plugin-rsc does in the client `generateBundle`: emit every CSS file
       * the server chunks import (under the asset's own fileName), then read the
       * very same imports as the server pages' CSS dependencies.
       */
      function forwardLikePluginRsc(rscBundle: Bundle) {
        const imported = Object.values(rscBundle).flatMap((output) =>
          output.type === 'chunk'
            ? [...(output.viteMetadata?.importedCss ?? [])]
            : [],
        )
        const emitted = [...new Set(imported)].map(
          (file) => rscBundle[file].fileName,
        )
        const dependencies = Object.fromEntries(
          Object.values(rscBundle).flatMap((output) =>
            output.type === 'chunk'
              ? [
                  [
                    output.fileName,
                    [...(output.viteMetadata?.importedCss ?? [])],
                  ],
                ]
              : [],
          ),
        )
        return { emitted, dependencies }
      }

      beforeEach(() => {
        getCssSpy.mockImplementation((fileNum: number | null) =>
          fileNum === null ? 'base sheet' : 'file sheet',
        )
      })

      it.each([
        {
          label: 'per-file css',
          singleCss: false,
          shared: ['base.css', 'file.css'],
        },
        { label: 'single css', singleCss: true, shared: ['base.css'] },
      ])(
        'keeps the server css dependencies and emits each file once ($label)',
        async ({ singleCss, shared }) => {
          const [plugin, restore] = createPlugins({ singleCss })
          const names: Record<string, string> = {
            'base.css': 'devup-ui.css',
            'file.css': 'devup-ui-3.css',
          }
          const serverBundle: Bundle = {
            'base.css': asset('base.css', names['base.css']),
            'file.css': asset('file.css', names['file.css']),
            'server-only.css': asset('server-only.css', 'server-only.css'),
            'page.js': chunk('page.js', [...shared, 'server-only.css']),
          }
          const clientBundle: Bundle = Object.fromEntries(
            shared.map((file) => [file, asset(file, names[file])]),
          )
          await plugin.configResolved(
            rscConfig({ rsc: serverBundle, client: clientBundle }),
          )

          await plugin.generateBundle.call(serverEnv, {}, serverBundle)
          await plugin.generateBundle.call(clientEnv, {}, clientBundle)
          const { emitted, dependencies } = forwardLikePluginRsc(serverBundle)

          expect(dependencies['page.js']).toEqual([
            ...shared,
            'server-only.css',
          ])
          expect(emitted).toHaveLength(new Set(emitted).size)
          expect(emitted.filter((name) => name in clientBundle)).toEqual([])
          expect(emitted).toContain('server-only.css')

          for (const name of emitted) clientBundle[name] = asset(name, name)
          restore.generateBundle.handler({}, clientBundle)

          expect(Object.keys(clientBundle).sort()).toEqual(
            [...shared, 'server-only.css'].sort(),
          )
          for (const file of shared) {
            expect(serverBundle[file].fileName).toBe(file)
          }
          expect(dependencies['page.js']).toEqual([
            ...(serverBundle['page.js'] as Chunk).viteMetadata!.importedCss!,
          ])
          expect(clientBundle['base.css']).toHaveProperty(
            'source',
            'base sheet',
          )
        },
      )

      it('undoes the stand-ins only once', async () => {
        const [plugin, restore] = createPlugins()
        const serverBundle: Bundle = {
          'base.css': asset('base.css', 'devup-ui.css'),
          'page.js': chunk('page.js', ['base.css']),
        }
        const clientBundle: Bundle = {
          'base.css': asset('base.css', 'devup-ui.css'),
        }
        await plugin.configResolved(rscConfig({ rsc: serverBundle }))
        await plugin.generateBundle.call(clientEnv, {}, clientBundle)
        expect(serverBundle['base.css'].fileName).not.toBe('base.css')

        restore.generateBundle.handler({}, clientBundle)
        expect(serverBundle['base.css'].fileName).toBe('base.css')

        serverBundle['base.css'].fileName = 'changed.css'
        restore.generateBundle.handler({}, clientBundle)
        expect(serverBundle['base.css'].fileName).toBe('changed.css')
      })

      it('leaves css the client does not emit to plugin-rsc as it is', async () => {
        const [plugin] = createPlugins()
        const serverBundle: Bundle = {
          'devup-ui-3.server.css': asset(
            'devup-ui-3.server.css',
            'devup-ui-3.css',
          ),
          'page.js': chunk('page.js', ['devup-ui-3.server.css']),
        }
        const clientBundle: Bundle = {
          'devup-ui-3.client.css': asset(
            'devup-ui-3.client.css',
            'devup-ui-3.css',
          ),
        }
        await plugin.configResolved(rscConfig({ rsc: serverBundle }))

        await plugin.generateBundle.call(clientEnv, {}, clientBundle)

        expect(forwardLikePluginRsc(serverBundle).emitted).toEqual([
          'devup-ui-3.server.css',
        ])
      })

      it('does not park a server entry that is not an asset', async () => {
        const [plugin] = createPlugins()
        const serverBundle: Bundle = {
          'shared.css': chunk('shared.css', []),
          'page.js': chunk('page.js', ['shared.css']),
          'plain.js': { type: 'chunk', fileName: 'plain.js', name: 'plain' },
        }
        const clientBundle: Bundle = {
          'shared.css': asset('shared.css', 'shared.css'),
        }
        await plugin.configResolved(rscConfig({ rsc: serverBundle }))

        await plugin.generateBundle.call(clientEnv, {}, clientBundle)

        expect(serverBundle['shared.css'].fileName).toBe('shared.css')
      })

      it('does not touch the client bundle the rsc plugin tracks', async () => {
        const [plugin] = createPlugins()
        const trackedClient: Bundle = {
          'base.css': asset('base.css', 'devup-ui.css'),
          'page.js': chunk('page.js', ['base.css']),
        }
        const clientBundle: Bundle = {
          'base.css': asset('base.css', 'devup-ui.css'),
        }
        await plugin.configResolved(rscConfig({ client: trackedClient }))

        await plugin.generateBundle.call(clientEnv, {}, clientBundle)

        expect(trackedClient['base.css'].fileName).toBe('base.css')
      })

      it.each([
        { label: 'no config', config: undefined },
        { label: 'no rsc plugin', config: { plugins: [{ name: 'other' }] } },
        {
          label: 'no api',
          config: { plugins: [{ name: 'rsc:minimal' }] },
        },
        {
          label: 'no manager bundles',
          config: { plugins: [{ name: 'rsc:minimal', api: { manager: {} } }] },
        },
      ])(
        'does nothing when plugin-rsc exposes no bundles ($label)',
        async ({ config }) => {
          const [plugin, restore] = createPlugins()
          const clientBundle: Bundle = {
            'base.css': asset('base.css', 'devup-ui.css'),
          }
          await plugin.configResolved(config)

          await plugin.generateBundle.call(clientEnv, {}, clientBundle)
          restore.generateBundle.handler({}, clientBundle)

          expect(Object.keys(clientBundle)).toEqual(['base.css'])
        },
      )

      it('ignores no-write analysis bundles', async () => {
        const [plugin] = createPlugins()
        const serverBundle: Bundle = {
          'base.css': asset('base.css', 'devup-ui.css'),
          'page.js': chunk('page.js', ['base.css']),
        }
        const clientBundle: Bundle = {
          'base.css': asset('base.css', 'devup-ui.css'),
        }
        await plugin.configResolved(rscConfig({ rsc: serverBundle }))

        await plugin.generateBundle.call(
          {
            environment: {
              name: 'client',
              config: { consumer: 'client', build: { write: false } },
            },
          },
          {},
          clientBundle,
        )

        expect(serverBundle['base.css'].fileName).toBe('base.css')
      })

      it('does not forward server css that the client already emits', async () => {
        const [plugin, restore] = createPlugins({})
        const base = asset('base.css', 'devup-ui.css')
        const file = asset('file.css', 'devup-ui-3.css')
        const entry = chunk('entry.js', [
          'base.css',
          'file.css',
          'server-only.css',
        ])
        const serverBundle: Bundle = {
          'base.css': base,
          'file.css': file,
          'server-only.css': asset('server-only.css', 'server-only.css'),
          'entry.js': entry,
        }
        const clientBase = asset('base.css', 'devup-ui.css')
        const clientFile = asset('file.css', 'devup-ui-3.css')
        const clientBundle: Bundle = {
          'base.css': clientBase,
          'file.css': clientFile,
        }
        await plugin.configResolved(rscConfig({ rsc: serverBundle }))

        await plugin.generateBundle.call(serverEnv, {}, serverBundle)
        await plugin.generateBundle.call(clientEnv, {}, clientBundle)
        const forwarded = forwardLikePluginRsc(serverBundle)

        expect(base.source).toEqual('base sheet')
        expect(file.source).toEqual('file sheet')
        expect(clientBase.source).toEqual('base sheet')
        expect(clientFile.source).toEqual('file sheet')
        expect(entry.viteMetadata?.importedCss).toEqual(
          new Set(['base.css', 'file.css', 'server-only.css']),
        )
        expect(forwarded.emitted).toEqual([
          'base.css.devup-forwarded',
          'file.css.devup-forwarded',
          'server-only.css',
        ])
        for (const name of forwarded.emitted) {
          clientBundle[name] = asset(name, name)
        }
        restore.generateBundle.handler({}, clientBundle)

        expect(base.fileName).toBe('base.css')
        expect(file.fileName).toBe('file.css')
        expect(clientBundle['base.css']).toBe(clientBase)
        expect(clientBundle['file.css']).toBe(clientFile)
        expect(Object.keys(clientBundle).sort()).toEqual([
          'base.css',
          'file.css',
          'server-only.css',
        ])
        expect(entry.viteMetadata?.importedCss).toEqual(
          new Set(['base.css', 'file.css', 'server-only.css']),
        )
      })

      it('ignores no-write analysis bundles when tracking server css', async () => {
        const [plugin, restore] = createPlugins({})
        const file = asset('file.css', 'devup-ui-3.css')
        const entry = chunk('entry.js', ['file.css'])
        const serverBundle: Bundle = {
          'file.css': file,
          'entry.js': entry,
        }
        await plugin.configResolved(rscConfig({}))
        await plugin.generateBundle.call(
          {
            environment: {
              name: 'rsc',
              config: { consumer: 'server', build: { write: false } },
            },
          },
          {},
          serverBundle,
        )
        const clientBundle: Bundle = {
          'file.css': asset('file.css', 'devup-ui-3.css'),
        }

        await plugin.generateBundle.call(clientEnv, {}, clientBundle)
        restore.generateBundle.handler({}, clientBundle)

        expect(entry.viteMetadata?.importedCss).toEqual(new Set(['file.css']))
        expect(file.fileName).toBe('file.css')
        expect(file.source).toBe('file sheet')
        expect(Object.keys(clientBundle)).toEqual(['file.css'])
      })

      it('keeps server forwarding for a different output file name', async () => {
        const [plugin, restore] = createPlugins({})
        const file = asset('devup-ui-3.server.css', 'devup-ui-3.css')
        const entry = chunk('entry.js', ['devup-ui-3.server.css'])
        const serverBundle: Bundle = {
          'devup-ui-3.server.css': file,
          'entry.js': entry,
        }
        const clientBundle: Bundle = {
          'devup-ui-3.client.css': asset(
            'devup-ui-3.client.css',
            'devup-ui-3.css',
          ),
        }
        await plugin.configResolved(rscConfig({ rsc: serverBundle }))
        await plugin.generateBundle.call(serverEnv, {}, serverBundle)

        await plugin.generateBundle.call(clientEnv, {}, clientBundle)
        const forwarded = forwardLikePluginRsc(serverBundle)
        for (const name of forwarded.emitted) clientBundle[name] = file
        restore.generateBundle.handler({}, clientBundle)

        expect(entry.viteMetadata?.importedCss).toEqual(
          new Set(['devup-ui-3.server.css']),
        )
        expect(forwarded.emitted).toEqual(['devup-ui-3.server.css'])
        expect(file.fileName).toBe('devup-ui-3.server.css')
        expect(file.source).toBe('file sheet')
        expect(Object.keys(clientBundle).sort()).toEqual([
          'devup-ui-3.client.css',
          'devup-ui-3.server.css',
        ])
      })
    })

    it('starts a build from its own options and ends it at closeBundle', async () => {
      const resetSpy = spyOn(wasm, 'resetBuildState').mockReturnValue(undefined)
      try {
        const first = createPlugin({})
        resetSpy.mockClear()
        createPlugin({})
        expect(resetSpy).not.toHaveBeenCalled()
        first.closeBundle()
        first.closeBundle()
        createPlugin({})
        expect(resetSpy).not.toHaveBeenCalled()
      } finally {
        resetSpy.mockRestore()
      }
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

  describe('Tailwind CSS definitions', () => {
    let directory: string
    let tailwindFile: string

    beforeEach(() => {
      directory = fs.mkdtempSync(join(os.tmpdir(), 'devup-vite-tailwind-'))
      tailwindFile = join(directory, 'tailwind.css')
      fs.writeFileSync(tailwindFile, '@theme { --color-brand: #0af; }')
      existsSyncSpy.mockReturnValue(true)
      readFileSpy.mockResolvedValue(
        JSON.stringify({ tailwind: { css: tailwindFile } }),
      )
    })

    afterEach(() => {
      fs.rmSync(directory, { recursive: true, force: true })
    })

    it('registers the text of the Tailwind CSS with the theme', async () => {
      const plugin = createPlugin({})
      await plugin.configResolved()
      expect(registerThemeSpy).toHaveBeenCalledWith({
        tailwindCss: '@theme { --color-brand: #0af; }',
      })
    })

    it('reads it again when the Tailwind CSS changes', async () => {
      const plugin = createPlugin({})
      await plugin.configResolved()
      registerThemeSpy.mockClear()
      fs.writeFileSync(tailwindFile, '@theme { --color-brand: #f00; }')

      await plugin.watchChange(tailwindFile)

      expect(registerThemeSpy).toHaveBeenCalledWith({
        tailwindCss: '@theme { --color-brand: #f00; }',
      })
    })

    it('compiles every module again when the Tailwind CSS changes', async () => {
      const plugin = createPlugin({})
      await plugin.configResolved()
      const first = {}
      const second = {}
      const environment = {
        ...createHotUpdateEnvironment('client'),
        moduleGraph: {
          invalidateModule: mock(),
          idToModuleMap: new Map([
            ['a', first],
            ['b', second],
          ]),
        },
      }

      const result = await plugin.hotUpdate.call(
        { environment },
        { file: tailwindFile, modules: [{}], timestamp: 1 },
      )

      expect(environment.moduleGraph.invalidateModule).toHaveBeenCalledTimes(2)
      expect(environment.moduleGraph.invalidateModule).toHaveBeenCalledWith(
        first,
        expect.any(Set),
        1,
        true,
      )
      expect(environment.hot.send).toHaveBeenCalledWith({ type: 'full-reload' })
      expect(result).toEqual([])
    })

    it('compiles every module again on a Vite 5 hot update', async () => {
      const plugin = createPlugin({})
      await plugin.configResolved()
      const invalidateModule = mock()
      const send = mock()
      const module = {}

      const result = await plugin.handleHotUpdate({
        file: tailwindFile,
        server: {
          moduleGraph: {
            invalidateModule,
            idToModuleMap: new Map([['a', module]]),
          },
          ws: { send },
        },
        modules: [],
        timestamp: 2,
      })

      expect(invalidateModule).toHaveBeenCalledWith(
        module,
        expect.any(Set),
        2,
        true,
      )
      expect(send).toHaveBeenCalledWith({ type: 'full-reload' })
      expect(result).toEqual([])
    })

    it('leaves a Tailwind CSS that is gone alone', async () => {
      const plugin = createPlugin({})
      await plugin.configResolved()
      registerThemeSpy.mockClear()
      existsSyncSpy.mockReturnValue(false)

      await plugin.watchChange(tailwindFile)

      expect(registerThemeSpy).not.toHaveBeenCalled()
    })
  })
  function createHotUpdateEnvironment(consumer: 'client' | 'server') {
    return {
      config: { consumer },
      moduleGraph: { invalidateModule: mock() },
      hot: { send: mock() },
    }
  }

  it('should invalidate and reload the client on a devup.json change', async () => {
    writeFileSpy.mockResolvedValueOnce(undefined)
    getThemeInterfaceSpy.mockReturnValue('interface code')
    existsSyncSpy.mockReturnValue(true)
    readFileSpy.mockResolvedValueOnce(JSON.stringify({ theme: 'theme' }))
    const environment = createHotUpdateEnvironment('client')
    const module = {}
    const plugin = createPlugin({})

    const result = await plugin.hotUpdate.call(
      { environment },
      { file: 'devup.json', modules: [module], timestamp: 1 },
    )

    expect(writeFileSpy).toHaveBeenCalledWith(
      join('df', 'theme.d.ts'),
      'interface code',
      'utf-8',
    )
    expect(environment.moduleGraph.invalidateModule).toHaveBeenCalledWith(
      module,
      expect.any(Set),
      1,
      true,
    )
    expect(environment.hot.send).toHaveBeenCalledWith({ type: 'full-reload' })
    expect(result).toEqual([])
  })

  it.each([
    ['an unrelated file', 'other.json'],
    // The client refreshes sheet contents through Vite's regular css HMR.
    ['a devup sheet', join(resolve('df', 'devup-ui'), 'devup-ui-3.css')],
  ])('should leave client hot updates of %s to vite', async (_name, file) => {
    existsSyncSpy.mockReturnValue(true)
    const environment = createHotUpdateEnvironment('client')
    const plugin = createPlugin({})

    const result = await plugin.hotUpdate.call(
      { environment },
      { file, modules: [{}], timestamp: 1 },
    )

    expect(result).toBeUndefined()
    expect(writeFileSpy).not.toHaveBeenCalledWith(
      join('df', 'theme.d.ts'),
      expect.any(String),
      'utf-8',
    )
    expect(environment.moduleGraph.invalidateModule).not.toHaveBeenCalled()
    expect(environment.hot.send).not.toHaveBeenCalled()
  })

  // A module runner cannot apply CSS: Vite would restart the render with a
  // full reload, and the modules transformed again would write again.
  it.each(['devup-ui.css', 'devup-ui-3.css'])(
    'should keep %s updates out of server environments',
    async (fileName) => {
      const environment = createHotUpdateEnvironment('server')
      const plugin = createPlugin({})

      const result = await plugin.hotUpdate.call(
        { environment },
        {
          file: join(resolve('df', 'devup-ui'), fileName),
          modules: [{}],
          timestamp: 1,
        },
      )

      expect(result).toEqual([])
      expect(environment.hot.send).not.toHaveBeenCalled()
    },
  )

  it.each([
    ['a source module', join(resolve('src'), 'App.tsx')],
    [
      'an app sheet named like a devup sheet',
      resolve('public', 'devup-ui.css'),
    ],
    ['devup.json', 'devup.json'],
  ])('should leave server hot updates of %s to vite', async (_name, file) => {
    existsSyncSpy.mockReturnValue(true)
    const environment = createHotUpdateEnvironment('server')
    const plugin = createPlugin({})

    const result = await plugin.hotUpdate.call(
      { environment },
      { file, modules: [{}], timestamp: 1 },
    )

    expect(result).toBeUndefined()
    expect(environment.hot.send).not.toHaveBeenCalled()
  })

  it('should print error when watch change error', async () => {
    writeFileSpy.mockResolvedValueOnce(undefined)
    getThemeInterfaceSpy.mockReturnValue('interface code')
    existsSyncSpy.mockImplementation(
      (path: fs.PathLike) => path === 'devup.json',
    )
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
        'css code',
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

  it('extracts a file that reaches Devup UI only through a project module', async () => {
    const source = `import { Box } from './ui'
export const a = <Box bg="red" />`
    codeExtractSpy.mockReturnValue(
      createCodeExtractResult({
        code: '<div className="a" />',
        css: '',
        cssFile: undefined,
        map: undefined,
        updatedBaseStyle: false,
      }),
    )
    const plugin = createPlugin({})

    expect(await plugin.transform(source, 'src/App.tsx')).toEqual({
      code: '<div className="a" />',
    })
    expect(codeExtractSpy.mock.calls.at(-1)?.[1]).toBe(source)
  })

  it('extracts a file that only the Emotion css prop compiles (pragma, no Devup UI import)', async () => {
    const source = `/** @jsxImportSource @emotion/react */
export const a = <div css={{ color: 'red' }} />`
    codeExtractSpy.mockReturnValue(
      createCodeExtractResult({
        code: '<div className="a" />',
        css: '',
        cssFile: undefined,
        map: undefined,
        updatedBaseStyle: false,
      }),
    )
    const plugin = createPlugin({})

    expect(await plugin.transform(source, 'src/App.tsx')).toEqual({
      code: '<div className="a" />',
    })
    expect(codeExtractSpy.mock.calls.at(-1)?.[1]).toBe(source)
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
        '@emotion/react': null,
        '@emotion/styled': 'styled',
        '@emotion/css': null,
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
        '@emotion/react': null,
        '@emotion/styled': 'styled',
        '@emotion/css': null,
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

  // Every write wakes the dev server's watcher, so a transform that leaves a
  // sheet unchanged must not touch it: the reload the write causes would
  // transform the module again, which would write again.
  it('writes a sheet only when its css changes', async () => {
    const plugin = createPlugin({})
    const sheet = join(resolve('df', 'devup-ui'), 'devup-ui-3.css')
    const transformWith = (css: string | undefined) => {
      codeExtractSpy.mockReturnValue(
        createCodeExtractResult({ css, cssFile: 'devup-ui-3.css' }),
      )
      return plugin.transform('code', 'foo.tsx')
    }

    await transformWith('.a{color:red}')
    await transformWith('.a{color:red}')
    // `css` is unset when the transform added no styles
    await transformWith(undefined)
    expect(writeFileSpy.mock.calls).toEqual([[sheet, '.a{color:red}', 'utf-8']])

    await transformWith('.a{color:red}.b{color:blue}')
    expect(writeFileSpy.mock.calls).toEqual([
      [sheet, '.a{color:red}', 'utf-8'],
      [sheet, '.a{color:red}.b{color:blue}', 'utf-8'],
    ])
  })

  it('writes the base sheet only when it changes', async () => {
    const plugin = createPlugin({})
    const baseSheet = join(resolve('df', 'devup-ui'), 'devup-ui.css')
    codeExtractSpy.mockReturnValue(
      createCodeExtractResult({ cssFile: '', updatedBaseStyle: true }),
    )

    getCssSpy.mockReturnValue('*{margin:0}')
    await plugin.transform('code', 'layout.tsx')
    await plugin.transform('code', 'layout.tsx')
    getCssSpy.mockReturnValue('*{margin:0}body{font-family:Pretendard}')
    await plugin.transform('code', 'layout.tsx')

    expect(writeFileSpy.mock.calls).toEqual([
      [baseSheet, '*{margin:0}', 'utf-8'],
      [baseSheet, '*{margin:0}body{font-family:Pretendard}', 'utf-8'],
    ])
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
    const [plugin] = DevupUI(options) as unknown as [
      { configResolved: ConfigResolved },
    ]
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

  it('does nothing when atomHoist is explicitly zero', async () => {
    await runConfigResolved({ atomHoist: 0 }, { root: '/p' })
    expect(buildCanonicalMapSpy).not.toHaveBeenCalled()
    expect(setAtomHoistSpy).not.toHaveBeenCalled()
  })

  describe('W66 default regressions', () => {
    beforeEach(() => {
      computeFileReachSpy.mockReturnValue({
        '/p/src/shared.tsx': [0, 1],
        '/p/src/a.tsx': [0],
        '/p/src/b.tsx': [1],
      })
    })

    it.each([undefined, 0, -1, NaN, Infinity, -Infinity, 0.5, 1, 2, 3])(
      'W66 availability: keeps per-file mode across environments for request %s',
      async (atomHoist) => {
        // Given: the same plugin is shared by Vite environment builds.
        const warningSpy = spyOn(console, 'warn').mockImplementation(() => {})
        const plugin = createPlugin({ atomHoist })
        try {
          // When: configuration and both environment transforms run.
          await plugin.configResolved({ root: '/p', command: 'build' })
          for (const consumer of ['client', 'server'] as const) {
            await plugin.transform.call(
              { environment: { name: consumer, config: { consumer } } },
              'code',
              '/p/src/a.tsx',
            )
          }
          // Then: no atom-only pre-pass or repeated warning is possible.
          expect(buildCanonicalMapSpy).not.toHaveBeenCalled()
          expect(computeFileReachSpy).not.toHaveBeenCalled()
          expect(importFileRoutesSpy).not.toHaveBeenCalled()
          expect(setAtomHoistSpy).not.toHaveBeenCalled()
          expect(warningSpy).toHaveBeenCalledTimes(
            atomHoist !== undefined &&
              Number.isFinite(atomHoist) &&
              atomHoist > 0
              ? 1
              : 0,
          )
        } finally {
          plugin.closeBundle()
          warningSpy.mockRestore()
        }
      },
    )

    it.each([
      ['omitted', {}],
      ['undefined', { atomHoist: undefined }],
    ])(
      'keeps omitted atomHoist %s silent without consulting route reach',
      async (_label, options) => {
        const warningSpy = spyOn(console, 'warn').mockImplementation(() => {})
        try {
          await runConfigResolved(options, { root: '/p' })
          expect(computeFileReachSpy).not.toHaveBeenCalled()
          expect(importFileRoutesSpy).not.toHaveBeenCalled()
          expect(setAtomHoistSpy).not.toHaveBeenCalled()
          expect(warningSpy).not.toHaveBeenCalled()
        } finally {
          warningSpy.mockRestore()
        }
      },
    )

    it.each([0, -1, NaN, Infinity, -Infinity])(
      'keeps legacy OFF for atomHoist %s',
      async (atomHoist) => {
        // Given: two routes, not a single-entry fallback.
        // When: an explicitly disabled numeric option is supplied.
        await runConfigResolved({ atomHoist }, { root: '/p' })
        // Then: the atom-only pre-pass has no effects.
        expect(buildCanonicalMapSpy).not.toHaveBeenCalled()
        expect(computeFileReachSpy).not.toHaveBeenCalled()
        expect(importFileRoutesSpy).not.toHaveBeenCalled()
        expect(setAtomHoistSpy).not.toHaveBeenCalled()
      },
    )

    it('defers a custom threshold above the minimum until reservation lands', async () => {
      // Given: two routes and a custom threshold.
      // When: config resolves.
      await runConfigResolved({ atomHoist: 3 }, { root: '/p' })
      // Then: unavailable atom hoisting never installs a threshold.
      expect(setAtomHoistSpy).not.toHaveBeenCalled()
      expect(computeFileReachSpy).not.toHaveBeenCalled()
    })

    it.each([
      ['omitted', {}],
      ['undefined', { atomHoist: undefined }],
      ['enabled', { atomHoist: 2 }],
    ])(
      'ignores %s atomHoist before planning in actual singleCss mode',
      async (_label, options) => {
        // Given: the real config/transform hooks under explicit OFF.
        const off = createPlugin({ singleCss: true, atomHoist: 0 })
        await off.configResolved({ root: '/p' })
        const offConfig = callConfig(off, ROLLUP_META)
        await off.transform('code', '/p/src/a.tsx')
        const offExtractions = [...codeExtractSpy.mock.calls]
        codeExtractSpy.mockClear()
        // When: only the requested atom policy changes.
        const on = createPlugin({ singleCss: true, ...options })
        await on.configResolved({ root: '/p' })
        await on.transform('code', '/p/src/a.tsx')
        const onConfig = callConfig(on, ROLLUP_META)
        // Then: no canonical/reach mutation and identical extraction IDs/import flags.
        expect(buildCanonicalMapSpy).not.toHaveBeenCalled()
        expect(importCanonicalMapSpy).not.toHaveBeenCalled()
        expect(computeFileReachSpy).not.toHaveBeenCalled()
        expect(importFileRoutesSpy).not.toHaveBeenCalled()
        expect(setAtomHoistSpy).not.toHaveBeenCalled()
        expect(codeExtractSpy.mock.calls).toEqual(offExtractions)
        expect(codeExtractSpy).toHaveBeenCalledWith(
          '/p/src/a.tsx',
          'code',
          '@devup-ui/react',
          expect.any(String),
          true,
          true,
          false,
          expect.anything(),
        )
        const cssId = resolve('df', 'devup-ui', 'devup-ui.css')
        expect(
          onConfig.build?.rollupOptions?.output?.manualChunks?.(cssId, ''),
        ).toBe(
          offConfig.build?.rollupOptions?.output?.manualChunks?.(cssId, ''),
        )
      },
    )
  })

  it('defers atom-only collapse and bucket folding until reservation lands', async () => {
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
    expect(buildCanonicalMapSpy).not.toHaveBeenCalled()
    expect(importCanonicalMapSpy).not.toHaveBeenCalled()
    expect(computeFileReachSpy).not.toHaveBeenCalled()
    expect(importFileRoutesSpy).not.toHaveBeenCalled()
    expect(setAtomHoistSpy).not.toHaveBeenCalled()
  })

  it.each([
    ['app', 'app'],
    ['src', 'src'],
  ])('defers atom reach in %s/ layouts', async (_name, dir) => {
    existsSyncSpy.mockImplementation(
      (path: string) => path === resolve('/p', dir),
    )
    computeFileReachSpy.mockReturnValue({
      '/p/a.tsx': [0],
      '/p/b.tsx': [1],
    })

    await runConfigResolved({ atomHoist: 2 }, { root: '/p' })

    expect(computeFileReachSpy).not.toHaveBeenCalled()
  })

  it('does not infer atom reach when neither conventional dir exists', async () => {
    existsSyncSpy.mockReturnValue(false)
    computeFileReachSpy.mockReturnValue({
      '/p/a.tsx': [0],
      '/p/b.tsx': [1],
    })

    await runConfigResolved({ atomHoist: 2 }, { root: '/p' })

    expect(computeFileReachSpy).not.toHaveBeenCalled()
  })

  it('defers even the former minimum-clamped threshold', async () => {
    computeFileReachSpy.mockReturnValue({
      '/p/src/a.tsx': [0],
      '/p/src/b.tsx': [1],
    })
    await runConfigResolved({ atomHoist: 1 }, { root: '/p' })
    expect(setAtomHoistSpy).not.toHaveBeenCalled()
    expect(computeFileReachSpy).not.toHaveBeenCalled()
  })

  it('stays off when fewer than two routes are reachable', async () => {
    computeFileReachSpy.mockReturnValue({ '/p/src/a.tsx': [0] })
    await runConfigResolved({ atomHoist: 2 }, { root: '/p' })
    expect(setAtomHoistSpy).not.toHaveBeenCalled()
  })

  it('does not infer atom reach from HTML entries before PR B', async () => {
    computeFileReachSpy.mockReturnValue({
      '/p/src/a.tsx': [0],
      '/p/src/b.tsx': [1],
    })
    await runConfigResolved(
      { atomHoist: 2 },
      { root: '/p', build: { rollupOptions: { input: 'index.html' } } },
    )
    expect(computeFileReachSpy).not.toHaveBeenCalled()
    expect(setAtomHoistSpy).not.toHaveBeenCalled()
  })

  it('keeps atom reach deferred for array and string JS entries', async () => {
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
    expect(computeFileReachSpy).not.toHaveBeenCalled()
    expect(setAtomHoistSpy).not.toHaveBeenCalled()
  })

  it('never reaches the unavailable atom pre-pass even if it would fail', async () => {
    buildCanonicalMapSpy.mockImplementation(() => {
      throw new Error('boom')
    })
    await runConfigResolved({ atomHoist: 2 }, { root: '/p' })
    expect(buildCanonicalMapSpy).not.toHaveBeenCalled()
    expect(setAtomHoistSpy).not.toHaveBeenCalled()
  })

  it('keeps atom reach deferred when config.root is absent', async () => {
    computeFileReachSpy.mockReturnValue({
      '/p/src/a.tsx': [0],
      '/p/src/b.tsx': [1],
    })
    await runConfigResolved({ atomHoist: 2 }, {})
    expect(computeFileReachSpy).not.toHaveBeenCalled()
    expect(setAtomHoistSpy).not.toHaveBeenCalled()
  })
})

describe('module resolver', () => {
  it('resolves imports to Vite ids and watches the modules read', async () => {
    const setModuleResolverSpy = spyOn(
      wasm,
      'setModuleResolver',
    ).mockReturnValue(undefined)
    try {
      codeExtractSpy.mockReturnValue(
        createCodeExtractResult({ dependencies: ['/p/src/tokens.ts'] }),
      )
      const plugin = createPlugin({})
      await plugin.configResolved({ root: '/p' })
      const resolveModule = setModuleResolverSpy.mock.calls[0][0] as (
        specifier: string,
        importer: string,
      ) => { path: string } | undefined
      expect(resolveModule('./plugin.test', import.meta.path)?.path).toBe(
        import.meta.path.replaceAll('\\', '/'),
      )
      const addWatchFile = mock()
      await plugin.transform.call(
        { addWatchFile } as never,
        'code',
        '/p/src/App.tsx',
      )
      expect(addWatchFile).toHaveBeenCalledWith('/p/src/tokens.ts')
    } finally {
      setModuleResolverSpy.mockRestore()
    }
  })
})
