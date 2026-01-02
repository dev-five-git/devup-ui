import * as fs from 'node:fs'
import * as fsPromises from 'node:fs/promises'
import { join, resolve } from 'node:path'

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

beforeEach(() => {
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

afterEach(() => {
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
    await plugin.setup({
      transform,
      modifyRsbuildConfig,
    } as any)
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
    await plugin.setup({
      transform,
      modifyRsbuildConfig,
    } as any)
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
    await plugin.setup({
      transform,
      modifyRsbuildConfig,
    } as any)
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
    await plugin.setup({
      transform,
      modifyRsbuildConfig,
    } as any)
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
    await plugin.setup({
      transform,
    } as any)
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
    await plugin.setup({
      transform,
      modifyRsbuildConfig,
    } as any)
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
    await plugin.setup({
      transform,
      modifyRsbuildConfig,
    } as any)
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
    codeExtractSpy.mockReturnValue({
      code: '<div></div>',
      css: '',
      css_file: 'devup-ui.css',
    } as any)
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
    await plugin.setup({
      transform,
      modifyRsbuildConfig: mock(),
    } as any)
    expect(transform).toHaveBeenCalled()
    expect(transform).toHaveBeenCalledWith(
      {
        test: /\.(tsx|ts|js|mjs|jsx)$/,
      },
      expect.any(Function),
    )
    codeExtractSpy.mockReturnValue({
      code: '<div></div>',
      css: '.devup-ui-1 { color: red; }',
      cssFile: 'devup-ui.css',
      map: undefined,
      updatedBaseStyle: options.updatedBaseStyle,
      free: mock(),
      [Symbol.dispose]: mock(),
    })
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
    await (plugin as any).setup({
      transform: mock(),
      renderChunk: mock(),
      generateBundle: mock(),
      closeBundle: mock(),
      resolve: mock(),
      load: mock(),
      modifyRsbuildConfig: mock(),
      watchChange: mock(),
      resolveId: mock(),
    } as any)
    if (options.existsDevupFile) {
      expect(readFileSpy).toHaveBeenCalledWith('devup.json', 'utf-8')
      expect(registerThemeSpy).toHaveBeenCalledWith({})
      expect(getThemeInterfaceSpy).toHaveBeenCalledWith(
        '@devup-ui/react',
        'CustomColors',
        'DevupThemeTypography',
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
    await (plugin as any).setup({
      transform: mock(),
      renderChunk: mock(),
      generateBundle: mock(),
      closeBundle: mock(),
      resolve: mock(),
      modifyRsbuildConfig,
      load: mock(),
      watchChange: mock(),
      resolveId: mock(),
    } as any)
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
    await plugin.setup({
      transform: mock(),
      modifyRsbuildConfig: mock(),
    } as any)
    expect(setPrefixSpy).toHaveBeenCalledWith('my-prefix')
  })
})
