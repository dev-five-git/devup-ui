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

function createCodeExtractResult(
  overrides: Partial<CodeExtractResult> = {},
): CodeExtractResult {
  return {
    code: '<div></div>',
    css: '',
    cssFile: 'devup-ui.css',
    map: undefined,
    updatedBaseStyle: false,
    free: mock(),
    [Symbol.dispose]: mock(),
    ...overrides,
  } as unknown as CodeExtractResult
}

function createSetupContext(
  overrides: Partial<RsbuildSetupContext> = {},
): RsbuildSetupContext {
  return {
    transform: mock(),
    modifyRsbuildConfig: mock(),
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

beforeAll(() => {
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
  it('should export DevupUIRsbuildPlugin', () => {
    expect(DevupUI).toBeDefined()
  })

  it('should be a function', () => {
    expect(DevupUI).toBeInstanceOf(Function)
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
      if (path === 'devup.json') return true
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
      if (path === 'devup.json') return true
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
    const originalConsoleError = console.error
    console.error = mock()
    readFileSpy.mockRejectedValueOnce('error')
    existsSyncSpy.mockImplementation((path: string) => {
      if (path === 'devup.json') return true
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
    expect(console.error).toHaveBeenCalledWith('error')
    console.error = originalConsoleError
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
        test: /\.(tsx|ts|js|mjs|jsx)$/,
      },
      expect.any(Function),
    )

    expect(
      transform.mock.calls[0][1]({
        code: `
                .devup-ui-1 {
                    color: red;
                }
            `,
      }),
    ).toBe('')
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
        test: /\.(tsx|ts|js|mjs|jsx)$/,
      },
      expect.any(Function),
    )

    expect(
      transform.mock.calls[0][1]({
        code: ``,
      }),
    ).toBe('')
    codeExtractSpy.mockReturnValue(
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
        test: /\.(tsx|ts|js|mjs|jsx)$/,
      },
      expect.any(Function),
    )
    codeExtractSpy.mockReturnValue(
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

    if (options.updatedBaseStyle) {
      expect(writeFileSpy).toHaveBeenCalledWith(
        resolve('df', 'devup-ui', 'devup-ui.css'),
        expect.stringMatching(/\/\* src\/App\.tsx \d+ \*\//),
        'utf-8',
      )
    }
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
      if (path === 'devup.json') return options.existsDevupFile
      if (path === 'df') return options.existsDistDir
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

    it('swallows pre-pass errors (atom hoisting stays off)', async () => {
      spies()
      buildCanonicalMapSpy.mockImplementation(() => {
        throw new Error('boom')
      })
      await DevupUI({ atomHoist: 2 }).setup(
        createSetupContext({ transform: mock(), modifyRsbuildConfig: mock() }),
      )
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
      })
      expect(servedChunk).toBe('CSS_FOR_3')
      expect(getCssSpy).toHaveBeenCalledWith(3, false)
      const servedBase = transform.mock.calls[0][1]({
        code: '',
        resourcePath: resolve('df', 'devup-ui', 'devup-ui.css'),
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
      codeExtractSpy.mockReturnValue(
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
      expect(rspackCfg2.optimization?.splitChunks).toBeUndefined()
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
})
