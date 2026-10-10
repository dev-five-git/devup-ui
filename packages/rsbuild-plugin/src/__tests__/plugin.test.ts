import * as fs from 'node:fs'
import * as fsPromises from 'node:fs/promises'
import { join, resolve } from 'node:path'

import * as pluginUtils from '@devup-ui/plugin-utils'
import * as wasm from '@devup-ui/wasm'
import {
  afterAll,
  afterEach,
  beforeAll,
  describe,
  expect,
  it,
  mock,
  spyOn,
} from 'bun:test'

import { DevupUI } from '../plugin'

type CodeExtractResult = ReturnType<typeof wasm.codeExtract>
type RsbuildPlugin = ReturnType<typeof DevupUI>
type RsbuildSetupContext = Parameters<RsbuildPlugin['setup']>[0]
const closeCallbacks: (() => void)[] = []
afterEach(() => {
  for (const close of closeCallbacks.splice(0)) close()
})

function createCodeExtractResult(
  overrides: Partial<CodeExtractResult> = {},
  events: string[] = [],
): CodeExtractResult {
  const values = {
    code: '<div></div>',
    css: '',
    cssFile: 'devup-ui.css',
    map: undefined,
    updatedBaseStyle: false,
    dependencies: [],
    ...overrides,
  }
  let live = true
  const free = mock(() => {
    expect(live).toBe(true)
    live = false
    events.push('free')
  })
  return {
    get code() {
      expect(live).toBe(true)
      events.push('code')
      return values.code
    },
    get css() {
      expect(live).toBe(true)
      events.push('css')
      return values.css
    },
    get map() {
      expect(live).toBe(true)
      events.push('map')
      return values.map
    },
    get cssFile() {
      expect(live).toBe(true)
      events.push('cssFile')
      return values.cssFile
    },
    get updatedBaseStyle() {
      expect(live).toBe(true)
      events.push('updatedBaseStyle')
      return values.updatedBaseStyle
    },
    get dependencies() {
      expect(live).toBe(true)
      events.push('dependencies')
      return values.dependencies
    },
    free,
    [Symbol.dispose]: free,
  }
}

function createSetupContext(
  overrides: Partial<RsbuildSetupContext> = {},
): RsbuildSetupContext {
  return {
    transform: mock(),
    modifyRsbuildConfig: mock(),
    modifyRspackConfig: mock((callback) => {
      const config: { plugins?: { apply(compiler: unknown): void }[] } = {}
      callback(config, { environment: { name: 'web' } })
      config.plugins?.[0]?.apply({
        options: {},
        hooks: { run: { tap: mock() }, thisCompilation: { tap: mock() } },
      })
    }),
    onBeforeBuild: mock(),
    onCloseBuild: mock((close) => closeCallbacks.push(close)),
    context: { rootPath: process.cwd() },
    renderChunk: mock(),
    generateBundle: mock(),
    closeBundle: mock(),
    resolve: mock(),
    load: mock(),
    watchChange: mock(),
    resolveId: mock(),
    ...overrides,
  } as unknown as RsbuildSetupContext
}

let existsSyncSpy: ReturnType<typeof spyOn>
let writeFileSyncSpy: ReturnType<typeof spyOn>
let mkdirSpy: ReturnType<typeof spyOn>
let readFileSpy: ReturnType<typeof spyOn>
let writeFileSpy: ReturnType<typeof spyOn>
let codeExtractSpy: ReturnType<typeof spyOn>
let getDefaultThemeSpy: ReturnType<typeof spyOn>
let getThemeInterfaceSpy: ReturnType<typeof spyOn>
let registerThemeSpy: ReturnType<typeof spyOn>
let setDebugSpy: ReturnType<typeof spyOn>
let setPrefixSpy: ReturnType<typeof spyOn>
let realpathSpy: ReturnType<typeof spyOn>

beforeAll(() => {
  realpathSpy = spyOn(fs, 'realpathSync').mockImplementation((path) =>
    resolve(String(path)),
  )
  existsSyncSpy = spyOn(fs, 'existsSync').mockReturnValue(false)
  writeFileSyncSpy = spyOn(fs, 'writeFileSync').mockReturnValue(undefined)
  mkdirSpy = spyOn(fsPromises, 'mkdir').mockResolvedValue(undefined)
  readFileSpy = spyOn(fsPromises, 'readFile').mockResolvedValue('{}')
  writeFileSpy = spyOn(fsPromises, 'writeFile').mockResolvedValue(undefined)
  codeExtractSpy = spyOn(wasm, 'codeExtract')
  getDefaultThemeSpy = spyOn(wasm, 'getDefaultTheme').mockReturnValue('')
  getThemeInterfaceSpy = spyOn(wasm, 'getThemeInterface').mockReturnValue('')
  registerThemeSpy = spyOn(wasm, 'registerTheme').mockReturnValue(undefined)
  setDebugSpy = spyOn(wasm, 'setDebug').mockReturnValue(undefined)
  setPrefixSpy = spyOn(wasm, 'setPrefix').mockReturnValue(undefined)
})

afterAll(() => {
  realpathSpy.mockRestore()
  existsSyncSpy.mockRestore()
  writeFileSyncSpy.mockRestore()
  mkdirSpy.mockRestore()
  readFileSpy.mockRestore()
  writeFileSpy.mockRestore()
  codeExtractSpy.mockRestore()
  getDefaultThemeSpy.mockRestore()
  getThemeInterfaceSpy.mockRestore()
  registerThemeSpy.mockRestore()
  setDebugSpy.mockRestore()
  setPrefixSpy.mockRestore()
})

describe('DevupUIRsbuildPlugin', () => {
  it.each([
    undefined,
    'code',
    'map',
    'cssFile',
    'updatedBaseStyle',
    'dependencies',
    'acquire',
    'consumer',
  ])(
    'releases Rsbuild Output before consumers when fault %s occurs',
    async (fault) => {
      const events: string[] = []
      const output = createCodeExtractResult(
        { cssFile: undefined, dependencies: ['dependency.ts'] },
        events,
      )
      const error = new Error('ownership fault')
      if (fault && fault !== 'acquire' && fault !== 'consumer')
        Object.defineProperty(output, fault, {
          get() {
            events.push(fault)
            throw error
          },
        })
      codeExtractSpy.mockImplementation(() => {
        if (fault === 'acquire') throw error
        return output
      })
      const transform = mock()
      await DevupUI().setup(createSetupContext({ transform }))
      const run = transform.mock.calls[1][1]({
        code: 'source',
        resourcePath: '/src/output.tsx',
        addDependency() {
          expect(output.free).toHaveBeenCalledTimes(1)
          if (fault === 'consumer') throw error
        },
      })
      if (fault) await expect(run).rejects.toBe(error)
      else expect(await run).toEqual({ code: '<div></div>', map: undefined })
      const fields = [
        'code',
        'map',
        'cssFile',
        'updatedBaseStyle',
        'dependencies',
      ]
      expect(events).toEqual(
        fault === 'acquire'
          ? []
          : [
              ...fields.slice(
                0,
                fault && fields.includes(fault)
                  ? fields.indexOf(fault) + 1
                  : fields.length,
              ),
              'free',
            ],
      )
      expect(output.free).toHaveBeenCalledTimes(fault === 'acquire' ? 0 : 1)
    },
  )
  it('does not mutate shorthands when only creating a configuration', () => {
    // Given
    const register = spyOn(wasm, 'registerShorthands').mockReturnValue(
      undefined,
    )
    try {
      // When
      DevupUI({ shorthands: { insetX: ['left', 'right'] } })
      // Then
      expect(register).not.toHaveBeenCalled()
    } finally {
      register.mockRestore()
    }
  })

  it('should export DevupUIRsbuildPlugin', () => {
    expect(DevupUI).toBeDefined()
  })

  it('should be a function', () => {
    expect(DevupUI).toBeInstanceOf(Function)
  })

  it('resolves imports to extraction names and depends on the modules read', async () => {
    const setModuleResolverSpy = spyOn(
      wasm,
      'setModuleResolver',
    ).mockReturnValue(undefined)
    const plugin = DevupUI()
    const transform = mock()
    await plugin.setup(createSetupContext({ transform }))
    const resolveModule = setModuleResolverSpy.mock.calls.at(-1)?.[0] as (
      specifier: string,
      importer: string,
    ) => { path: string } | undefined
    expect(resolveModule('./plugin.test', import.meta.path)?.path).toBe(
      import.meta.path,
    )
    codeExtractSpy.mockImplementation(() =>
      createCodeExtractResult({ dependencies: ['/src/tokens.ts'] }),
    )
    const addDependency = mock()
    await transform.mock.calls[1][1]({
      code: "import { Box } from '@devup-ui/react'",
      resourcePath: 'src/App.tsx',
      addDependency,
    })
    expect(addDependency).toHaveBeenCalledWith('/src/tokens.ts')
    setModuleResolverSpy.mockRestore()
  })

  it('should return a plugin object with correct name', async () => {
    const plugin = DevupUI()
    expect(plugin).toBeDefined()
    expect(plugin.name).toBe('devup-ui-rsbuild-plugin')
    expect(typeof plugin.setup).toBe('function')

    const transform = mock()
    const modifyRsbuildConfig = mock()
    await plugin.setup(
      createSetupContext({
        transform,
        modifyRsbuildConfig,
      }),
    )
    expect(transform).toHaveBeenCalled()
  })

  it('should write data files', async () => {
    readFileSpy.mockResolvedValueOnce(JSON.stringify({}))
    getThemeInterfaceSpy.mockReturnValue('interface code')
    existsSyncSpy.mockImplementation((path: string) => {
      if (path === resolve('devup.json')) return true
      return false
    })
    const plugin = DevupUI()
    expect(plugin).toBeDefined()
    expect(plugin.setup).toBeDefined()
    const transform = mock()
    const modifyRsbuildConfig = mock()
    await plugin.setup(
      createSetupContext({
        transform,
        modifyRsbuildConfig,
      }),
    )
  })

  it('should write data files without theme', async () => {
    readFileSpy.mockResolvedValueOnce(JSON.stringify({}))
    getThemeInterfaceSpy.mockReturnValue('')
    existsSyncSpy.mockImplementation((path: string) => {
      if (path === resolve('devup.json')) return true
      return false
    })
    const plugin = DevupUI()
    expect(plugin).toBeDefined()
    expect(plugin.setup).toBeDefined()
    const transform = mock()
    const modifyRsbuildConfig = mock()
    await plugin.setup(
      createSetupContext({
        transform,
        modifyRsbuildConfig,
      }),
    )
    expect(writeFileSyncSpy).not.toHaveBeenCalled()
  })

  it('should error when write data files', async () => {
    readFileSpy.mockRejectedValueOnce('error')
    existsSyncSpy.mockImplementation((path: string) => {
      if (path === resolve('devup.json')) return true
      return false
    })
    const plugin = DevupUI()
    expect(plugin).toBeDefined()
    expect(plugin.setup).toBeDefined()
    const transform = mock()
    const modifyRsbuildConfig = mock()
    await expect(
      plugin.setup(
        createSetupContext({
          transform,
          modifyRsbuildConfig,
        }),
      ),
    ).rejects.toThrow('error')
  })

  it('should not register css transform', async () => {
    const plugin = DevupUI({
      extractCss: false,
    })
    expect(plugin).toBeDefined()
    expect(plugin.setup).toBeDefined()
    const transform = mock()
    await plugin.setup(
      createSetupContext({
        transform,
      }),
    )
    expect(transform).not.toHaveBeenCalled()
  })

  it('should accept custom options', () => {
    const customOptions = {
      package: '@custom/devup-ui',
      cssFile: './custom.css',
      devupPath: './custom-df',
      interfacePath: './custom-interface',
      extractCss: false,
      debug: true,
      include: ['src/**/*'],
    }

    const plugin = DevupUI(customOptions)
    expect(plugin).toBeDefined()
    expect(plugin.name).toBe('devup-ui-rsbuild-plugin')
  })
  it('should transform css', async () => {
    const plugin = DevupUI()
    expect(plugin).toBeDefined()
    expect(plugin.setup).toBeDefined()
    const transform = mock()
    const modifyRsbuildConfig = mock()
    await plugin.setup(
      createSetupContext({
        transform,
        modifyRsbuildConfig,
      }),
    )
    expect(transform).toHaveBeenCalled()
    expect(transform).toHaveBeenCalledWith(
      {
        test: pluginUtils.SOURCE_FILE_RE,
      },
      expect.any(Function),
    )

    const getCssSpy = spyOn(wasm, 'getCss').mockReturnValue('file css')
    expect(
      transform.mock.calls[0][1]({
        code: '/* placeholder */',
        resourcePath: resolve('df', 'devup-ui', 'devup-ui-1.css'),
        environment: { name: 'web' },
      }),
    ).toBe('file css')
    // A file's stylesheet imports the shared base
    expect(getCssSpy).toHaveBeenCalledWith(1, true)
    getCssSpy.mockRestore()
  })
  it('should transform code', async () => {
    const plugin = DevupUI()
    expect(plugin).toBeDefined()
    expect(plugin.setup).toBeDefined()
    const transform = mock()
    const modifyRsbuildConfig = mock()
    await plugin.setup(
      createSetupContext({
        transform,
        modifyRsbuildConfig,
      }),
    )
    expect(transform).toHaveBeenCalled()
    expect(transform).toHaveBeenCalledWith(
      {
        test: pluginUtils.SOURCE_FILE_RE,
      },
      expect.any(Function),
    )

    codeExtractSpy.mockImplementation(() =>
      createCodeExtractResult({
        code: '<div></div>',
        css: '',
        cssFile: 'devup-ui.css',
      }),
    )
    await expect(
      transform.mock.calls[1][1]({
        code: `import { Box } from '@devup-ui/react'
const App = () => <Box></Box>`,
        resourcePath: 'src/App.tsx',
      }),
    ).resolves.toEqual({
      code: '<div></div>',
      map: undefined,
    })
    await expect(
      transform.mock.calls[1][1]({
        code: `import { Box } from '@devup-ui/react'
const App = () => <Box></Box>`,
        resourcePath: 'node_modules/@wrong-ui/react/index.tsx',
      }),
    ).resolves.toEqual(
      `import { Box } from '@devup-ui/react'
const App = () => <Box></Box>`,
    )
  })
  it.each(
    createTestMatrix({
      updatedBaseStyle: [true, false],
    }),
  )('should transform with include', async (options) => {
    const plugin = DevupUI({
      include: ['lib'],
    })
    expect(plugin).toBeDefined()
    expect(plugin.setup).toBeDefined()
    const transform = mock()
    await plugin.setup(
      createSetupContext({
        transform,
        modifyRsbuildConfig: mock(),
      }),
    )
    expect(transform).toHaveBeenCalled()
    expect(transform).toHaveBeenCalledWith(
      {
        test: pluginUtils.SOURCE_FILE_RE,
      },
      expect.any(Function),
    )
    codeExtractSpy.mockImplementation(() =>
      createCodeExtractResult({
        code: '<div></div>',
        css: '.devup-ui-1 { color: red; }',
        cssFile: 'devup-ui.css',
        map: undefined,
        updatedBaseStyle: options.updatedBaseStyle,
      }),
    )
    const ret = await transform.mock.calls[1][1]({
      code: `import { Box } from '@devup-ui/react'
const App = () => <Box></Box>`,
      resourcePath: 'src/App.tsx',
    })
    expect(ret).toEqual({
      code: '<div></div>',
      map: undefined,
    })

    expect(writeFileSpy).toHaveBeenCalledWith(
      resolve('df', 'devup-ui', 'devup-ui.css'),
      expect.stringMatching(/\/\* src\/App\.tsx \d+ \*\//),
      'utf-8',
    )

    const ret1 = await transform.mock.calls[1][1]({
      code: `import { Box } from '@devup-ui/react'
const App = () => <Box></Box>`,
      resourcePath: 'node_modules/@devup-ui/react/index.tsx',
    })
    expect(ret1).toEqual({
      code: `<div></div>`,
      map: undefined,
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
      getDefaultTheme: ['theme', ''],
      singleCss: [true, false],
    }),
  )('should write data files', async (options) => {
    writeFileSpy.mockResolvedValueOnce(undefined)
    readFileSpy.mockResolvedValueOnce(JSON.stringify({}))
    getThemeInterfaceSpy.mockReturnValue('interface code')
    getDefaultThemeSpy.mockReturnValue(options.getDefaultTheme)
    existsSyncSpy.mockImplementation((path: string) => {
      if (path === resolve('devup.json')) return options.existsDevupFile
      if (path === resolve('df')) return options.existsDistDir
      if (path === resolve('df', 'devup-ui')) return options.existsCssDir
      if (path === join('df', 'sheet.json')) return options.existsSheetFile
      if (path === join('df', 'classMap.json'))
        return options.existsClassMapFile
      if (path === join('df', 'fileMap.json')) return options.existsFileMapFile
      return false
    })
    const plugin = DevupUI({ singleCss: options.singleCss })
    await plugin.setup(createSetupContext())
    if (options.existsDevupFile) {
      expect(readFileSpy).toHaveBeenCalledWith(resolve('devup.json'), 'utf-8')
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
        resolve('df', 'theme.d.ts'),
        'interface code',
        'utf-8',
      )
    } else {
      expect(registerThemeSpy).toHaveBeenCalledWith({})
    }

    const modifyRsbuildConfig = mock()
    await plugin.setup(createSetupContext({ modifyRsbuildConfig }))
    if (options.getDefaultTheme) {
      expect(modifyRsbuildConfig).toHaveBeenCalledWith(expect.any(Function))
      const config = {
        source: {
          define: {},
        },
      }
      modifyRsbuildConfig.mock.calls[0][0](config)
      expect(config).toEqual({
        source: {
          define: {
            'process.env.DEVUP_UI_DEFAULT_THEME': JSON.stringify(
              options.getDefaultTheme,
            ),
          },
        },
      })
    } else {
      expect(modifyRsbuildConfig).toHaveBeenCalledWith(expect.any(Function))
      const config = {
        source: {
          define: {},
        },
      }
      modifyRsbuildConfig.mock.calls[0][0](config)
      expect(config).toEqual({
        source: {
          define: {},
        },
      })
    }
  })

  it('should call setPrefix when prefix option is provided', async () => {
    const plugin = DevupUI({ prefix: 'my-prefix' })
    await plugin.setup(
      createSetupContext({
        transform: mock(),
        modifyRsbuildConfig: mock(),
      }),
    )
    expect(setPrefixSpy).toHaveBeenCalledWith('my-prefix')
  })

  describe('deterministic file numbering', () => {
    it.each([
      ['relative', false],
      ['posix', true],
    ])('numbers the files the scan finds (%s ids)', async (_name, atomMode) => {
      const collectSpy = spyOn(pluginUtils, 'collectNumberedFiles')
      const seedSpy = spyOn(wasm, 'seedFileMap').mockReturnValue(undefined)
      const closeBuild = mock()
      try {
        collectSpy.mockImplementation((options: any) => {
          expect(options.toId('C:\\p\\a.tsx')).toBe(
            atomMode ? 'C:/p/a.tsx' : 'C:\\p\\a.tsx',
          )
          return ['/p/a.tsx']
        })
        await DevupUI({
          include: ['@acme/ui'],
          sourceDirs: ['src'],
          atomHoist: atomMode ? 2 : undefined,
        }).setup(createSetupContext({ onCloseBuild: closeBuild }))
        expect(seedSpy).toHaveBeenCalledWith(['/p/a.tsx'])
        expect(collectSpy.mock.calls[0][0]).toMatchObject({
          roots: [resolve('src')],
          include: ['@acme/ui'],
        })
        closeBuild.mock.calls[0][0]()
        collectSpy.mockImplementation(() => {
          throw new Error('scan boom')
        })
        await DevupUI().setup(createSetupContext())
      } finally {
        collectSpy.mockRestore()
        seedSpy.mockRestore()
      }
    })

    it('sets the prefix every time, even without one', async () => {
      setPrefixSpy.mockClear()
      await DevupUI().setup(createSetupContext())
      expect(setPrefixSpy).toHaveBeenCalledWith(null)
    })
  })
  describe('atomHoist pre-pass', () => {
    let buildCanonicalMapSpy: ReturnType<typeof spyOn>
    let computeFileReachSpy: ReturnType<typeof spyOn>
    let importCanonicalMapSpy: ReturnType<typeof spyOn>
    let importFileRoutesSpy: ReturnType<typeof spyOn>
    let setAtomHoistSpy: ReturnType<typeof spyOn>
    let getCssSpy: ReturnType<typeof spyOn>

    function spies() {
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
      getCssSpy = spyOn(wasm, 'getCss').mockReturnValue('CSS')
    }
    afterEach(() => {
      buildCanonicalMapSpy?.mockRestore()
      computeFileReachSpy?.mockRestore()
      importCanonicalMapSpy?.mockRestore()
      importFileRoutesSpy?.mockRestore()
      setAtomHoistSpy?.mockRestore()
      getCssSpy?.mockRestore()
    })

    it('does nothing when atomHoist is unset', async () => {
      spies()
      await DevupUI().setup(
        createSetupContext({ transform: mock(), modifyRsbuildConfig: mock() }),
      )
      expect(buildCanonicalMapSpy).not.toHaveBeenCalled()
      expect(setAtomHoistSpy).not.toHaveBeenCalled()
      expect(importFileRoutesSpy).not.toHaveBeenCalled()
    })

    it('composes collapse + hoist and folds reach onto the canonical bucket', async () => {
      spies()
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
      await DevupUI({ atomHoist: 2 }).setup(
        createSetupContext({ transform: mock(), modifyRsbuildConfig: mock() }),
      )
      // rsbuild passes absolute resourcePath -> keyBy absolute
      expect(buildCanonicalMapSpy).toHaveBeenCalledWith(
        expect.objectContaining({ keyBy: 'absolute' }),
      )
      expect(importCanonicalMapSpy).toHaveBeenCalled()
      expect(importFileRoutesSpy).toHaveBeenCalledWith({
        '/p/src/parent.tsx': [0, 1],
        '/p/src/r1.tsx': [1],
      })
      expect(setAtomHoistSpy).toHaveBeenCalledWith(2)
    })

    it('clamps the threshold to a minimum of 2', async () => {
      spies()
      computeFileReachSpy.mockReturnValue({
        '/p/src/a.tsx': [0],
        '/p/src/b.tsx': [1],
      })
      await DevupUI({ atomHoist: 1 }).setup(
        createSetupContext({ transform: mock(), modifyRsbuildConfig: mock() }),
      )
      expect(setAtomHoistSpy).toHaveBeenCalledWith(2)
    })

    it('stays off when fewer than two routes are reachable', async () => {
      spies()
      computeFileReachSpy.mockReturnValue({ '/p/src/a.tsx': [0] })
      await DevupUI({ atomHoist: 2 }).setup(
        createSetupContext({ transform: mock(), modifyRsbuildConfig: mock() }),
      )
      expect(setAtomHoistSpy).not.toHaveBeenCalled()
    })

    it('fails requested atom planning when the graph fails', async () => {
      spies()
      buildCanonicalMapSpy.mockImplementation(() => {
        throw new Error('boom')
      })
      await expect(
        DevupUI({ atomHoist: 2 }).setup(
          createSetupContext({
            transform: mock(),
            modifyRsbuildConfig: mock(),
          }),
        ),
      ).rejects.toThrow('boom')
      expect(setAtomHoistSpy).not.toHaveBeenCalled()
    })

    it('serves per-route getCss(fileNum) for css imports in atom mode', async () => {
      spies()
      computeFileReachSpy.mockReturnValue({
        '/p/src/a.tsx': [0],
        '/p/src/b.tsx': [1],
      })
      getCssSpy.mockImplementation(
        (fileNum: number | null) => `CSS_FOR_${String(fileNum)}`,
      )
      const transform = mock()
      await DevupUI({ atomHoist: 2 }).setup(
        createSetupContext({ transform, modifyRsbuildConfig: mock() }),
      )
      // calls[0] is the cssDir transform; route chunk + base served as separate
      // modules (the entry code imports both), so each is getCss(fileNum, false).
      const servedChunk = transform.mock.calls[0][1]({
        code: '',
        resourcePath: resolve('df', 'devup-ui', 'devup-ui-3.css'),
        environment: { name: 'web' },
      })
      expect(servedChunk).toBe('CSS_FOR_3')
      expect(getCssSpy).toHaveBeenCalledWith(3, false)
      const servedBase = transform.mock.calls[0][1]({
        code: '',
        resourcePath: resolve('df', 'devup-ui', 'devup-ui.css'),
        environment: { name: 'web' },
      })
      expect(servedBase).toBe('CSS_FOR_null')
      expect(getCssSpy).toHaveBeenCalledWith(null, false)
    })

    it('extracts with posix filename + relative cssDir in atom mode', async () => {
      spies()
      computeFileReachSpy.mockReturnValue({
        '/p/src/a.tsx': [0],
        '/p/src/b.tsx': [1],
      })
      codeExtractSpy.mockImplementation(() =>
        createCodeExtractResult({ code: '<div></div>', cssFile: '' }),
      )
      const transform = mock()
      await DevupUI({ atomHoist: 2 }).setup(
        createSetupContext({ transform, modifyRsbuildConfig: mock() }),
      )
      // calls[1] is the source transform; atom mode posix-normalizes the
      // filename and passes a relative cssDir + import_main_css_in_code=true.
      await transform.mock.calls[1][1]({
        code: `import { Box } from '@devup-ui/react'\nconst A = () => <Box w="1px" />`,
        resourcePath: 'src/App.tsx',
      })
      const call = codeExtractSpy.mock.calls.at(-1)!
      expect(call[0]).toBe('src/App.tsx') // posix-normalized (already posix here)
      expect(typeof call[3]).toBe('string')
      expect((call[3] as string).startsWith('./')).toBe(true) // relative cssDir
      expect(call[5]).toBe(true) // import_main_css_in_code
      expect(call[6]).toBe(false) // import_main_css_in_css
    })

    it('injects a shared-css splitChunks cacheGroup in atom mode', async () => {
      spies()
      computeFileReachSpy.mockReturnValue({
        '/p/src/a.tsx': [0],
        '/p/src/b.tsx': [1],
      })
      const modifyRsbuildConfig = mock()
      await DevupUI({ atomHoist: 2 }).setup(
        createSetupContext({ transform: mock(), modifyRsbuildConfig }),
      )
      // prev undefined -> tools.rspack is the single injector function
      const cfg = {} as { tools?: { rspack?: unknown } }
      modifyRsbuildConfig.mock.calls[0][0](cfg)
      const inject = cfg.tools?.rspack as (c: unknown) => void
      expect(typeof inject).toBe('function')
      // applying it adds the cacheGroup when splitChunks is an object
      const rspackCfg = {
        optimization: {
          splitChunks: {} as {
            cacheGroups?: Record<string, { type?: string }>
          },
        },
      }
      inject(rspackCfg)
      expect(
        rspackCfg.optimization.splitChunks.cacheGroups?.devupUiShared.type,
      ).toBe('css/mini-extract')
      // splitChunks missing/false -> no cacheGroup added, no throw
      const rspackCfg2 = {} as { optimization?: { splitChunks?: unknown } }
      inject(rspackCfg2)
      expect(rspackCfg2.optimization?.splitChunks).toMatchObject({
        cacheGroups: { devupUiShared: { type: 'css/mini-extract' } },
      })
      expect(() => inject({ optimization: { splitChunks: false } })).toThrow(
        'splitChunks: false',
      )
    })

    it('composes the cacheGroup with existing tools.rspack (function then array)', async () => {
      spies()
      computeFileReachSpy.mockReturnValue({
        '/p/src/a.tsx': [0],
        '/p/src/b.tsx': [1],
      })
      const modifyFn = mock()
      await DevupUI({ atomHoist: 2 }).setup(
        createSetupContext({
          transform: mock(),
          modifyRsbuildConfig: modifyFn,
        }),
      )
      const prevFn = mock()
      const cfgFn = { tools: { rspack: prevFn as unknown } }
      modifyFn.mock.calls[0][0](cfgFn)
      expect(Array.isArray(cfgFn.tools.rspack)).toBe(true)
      expect((cfgFn.tools.rspack as unknown[])[0]).toBe(prevFn)

      const modifyArr = mock()
      await DevupUI({ atomHoist: 2 }).setup(
        createSetupContext({
          transform: mock(),
          modifyRsbuildConfig: modifyArr,
        }),
      )
      const prevArr = [mock()] as unknown[]
      const cfgArr = { tools: { rspack: prevArr as unknown } }
      modifyArr.mock.calls[0][0](cfgArr)
      expect((cfgArr.tools.rspack as unknown[]).length).toBe(2)
    })
  })

  describe('stylesheets built too early', () => {
    let getCssSpy: ReturnType<typeof spyOn>
    let readFileSyncSpy: ReturnType<typeof spyOn>
    let computeReachableFilesSpy: ReturnType<typeof spyOn>

    afterEach(() => {
      getCssSpy.mockRestore()
      readFileSyncSpy.mockRestore()
      computeReachableFilesSpy.mockRestore()
      existsSyncSpy.mockReturnValue(false)
    })

    async function setup(options: Parameters<typeof DevupUI>[0] = {}) {
      getCssSpy = spyOn(wasm, 'getCss').mockReturnValue('before')
      readFileSyncSpy = spyOn(fs, 'readFileSync').mockReturnValue('source')
      computeReachableFilesSpy = spyOn(
        pluginUtils,
        'computeReachableFiles',
      ).mockReturnValue([resolve('src', 'App.tsx')])
      codeExtractSpy.mockImplementation(() => createCodeExtractResult())
      const transform = mock()
      const onBeforeBuild = mock()
      const modifyRspackConfig = mock()
      await DevupUI(options).setup(
        createSetupContext({ transform, onBeforeBuild, modifyRspackConfig }),
      )
      const config: { plugins?: { apply(compiler: unknown): void }[] } = {}
      modifyRspackConfig.mock.calls[0][0](config, {
        environment: { name: 'web' },
      })
      const taps: Record<string, (...args: unknown[]) => unknown> = {}
      const tap =
        (hook: string) =>
        (name: unknown, fn: (...args: unknown[]) => unknown) => {
          taps[name === 'DevupUICompiledSourceGuard' ? `${hook}:guard` : hook] =
            fn
        }
      const compiler = {
        options: {},
        watchMode: false,
        rspack: { Compilation: { PROCESS_ASSETS_STAGE_REPORT: 5000 } },
        hooks: {
          run: { tap: tap('run') },
          thisCompilation: { tap: tap('start') },
        },
      }
      config.plugins![0]!.apply(compiler)
      const compilation = {
        assets: { 'index.js': {} },
        fileDependencies: new Set<string>(),
        missingDependencies: new Set<string>(),
        deleteAsset: mock(),
        hooks: {
          finishModules: { tap: tap('finishModules') },
          processAssets: { tap: tap('processAssets') },
          needAdditionalPass: { tap: tap('needAdditionalPass') },
        },
      }
      const serve = (resourcePath: string) =>
        transform.mock.calls[0][1]({
          resourcePath,
          environment: { name: 'web' },
        })
      return { onBeforeBuild, compiler, compilation, taps, serve }
    }

    it('extracts the files the entries reach before building', async () => {
      const { onBeforeBuild } = await setup({ atomHoist: undefined })
      const discarded: { events: string[]; output: CodeExtractResult }[] = []
      codeExtractSpy.mockImplementation(() => {
        const events: string[] = []
        const output = createCodeExtractResult({}, events)
        discarded.push({ events, output })
        return output
      })
      codeExtractSpy.mockClear()
      onBeforeBuild.mock.calls[0][0]({
        environments: {
          web: {
            entry: {
              a: './src/a.tsx',
              b: ['./src/b.tsx'],
              c: { import: './src/c.tsx' },
              d: { import: ['./src/d.tsx'] },
            },
          },
        },
      })
      expect(computeReachableFilesSpy).toHaveBeenCalledWith(
        expect.objectContaining({
          srcDir: [],
          tsconfigPath: resolve(process.cwd(), 'tsconfig.json'),
          entries: [],
        }),
      )
      expect(codeExtractSpy).toHaveBeenCalledWith(
        resolve('src', 'App.tsx'),
        'source',
        '@devup-ui/react',
        expect.stringMatching(/^\.\//),
        false,
        false,
        true,
        expect.anything(),
      )
      expect(discarded.length).toBeGreaterThan(0)
      for (const { events, output } of discarded) {
        expect(events).toEqual(['free'])
        expect(output.free).toHaveBeenCalledTimes(1)
      }

      // an extraction error is reported by the transform of that file
      codeExtractSpy.mockImplementation(() => {
        throw new Error('boom')
      })
      expect(() =>
        onBeforeBuild.mock.calls[0][0]({ environments: {} }),
      ).toThrow('prewarm failed')
    })

    it('extracts under posix names in atom mode', async () => {
      const { onBeforeBuild } = await setup({ atomHoist: 2 })
      codeExtractSpy.mockClear()
      onBeforeBuild.mock.calls[0][0]({ environments: {} })
      expect(codeExtractSpy.mock.calls[0]![0]).toBe(
        resolve('src', 'App.tsx').replaceAll('\\', '/'),
      )
    })

    it('compiles once more, writing no file, when a stylesheet changed', async () => {
      const { compiler, compilation, taps, serve } = await setup()
      writeFileSyncSpy.mockClear()
      taps.start!(compilation)
      // the shared base is written first, as the CSS loaders read it from disk
      expect(writeFileSyncSpy).toHaveBeenCalledWith(
        resolve('df', 'devup-ui', 'devup-ui.css'),
        'before',
        'utf-8',
      )
      serve(resolve('df', 'devup-ui', 'devup-ui-1.css'))
      getCssSpy.mockReturnValue('after')
      taps.finishModules!()
      taps.processAssets!()
      expect(compilation.deleteAsset).toHaveBeenCalledWith('index.js')
      writeFileSyncSpy.mockClear()
      expect(taps.needAdditionalPass!()).toBe(true)
      expect(writeFileSyncSpy).toHaveBeenCalledWith(
        resolve('df', 'devup-ui', 'devup-ui-1.css'),
        'after',
        'utf-8',
      )
      expect(writeFileSyncSpy).toHaveBeenCalledWith(
        resolve('df', 'devup-ui', 'devup-ui.css'),
        'after',
        'utf-8',
      )

      // one more pass per run at most
      compilation.deleteAsset.mockClear()
      taps.processAssets!()
      expect(compilation.deleteAsset).not.toHaveBeenCalled()
      expect(taps.needAdditionalPass!()).toBe(false)
      taps.run!()
      expect(taps.needAdditionalPass!()).toBe(true)

      // the dev server rebuilds through the files the transforms write
      compiler.watchMode = true
      taps.start!(compilation)
      taps.finishModules!()
      expect(taps.needAdditionalPass!()).toBe(false)
    })

    it('keeps the pass when every stylesheet is current', async () => {
      const { compilation, taps, serve } = await setup()
      existsSyncSpy.mockReturnValue(true)
      readFileSyncSpy.mockReturnValue('before')
      writeFileSyncSpy.mockClear()
      taps.start!(compilation)
      expect(writeFileSyncSpy).not.toHaveBeenCalled()
      serve(resolve('df', 'devup-ui', 'devup-ui-1.css'))
      taps.finishModules!()
      taps.processAssets!()
      expect(compilation.deleteAsset).not.toHaveBeenCalled()
      expect(taps.needAdditionalPass!()).toBe(false)
    })
  })
})
