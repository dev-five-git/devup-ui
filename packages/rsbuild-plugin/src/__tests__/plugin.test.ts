import { existsSync, writeFileSync } from 'node:fs'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { join, resolve } from 'node:path'

import {
  codeExtract,
  getDefaultTheme,
  getThemeInterface,
  registerTheme,
  setPrefix,
} from '@devup-ui/wasm'
import type { Mock } from 'bun:test'
import { beforeEach, describe, expect, it, mock } from 'bun:test'

import { DevupUI } from '../plugin'

// Mock dependencies
mock.module('node:fs/promises', () => ({
  mkdir: mock(),
  readFile: mock(),
  writeFile: mock(),
}))
mock.module('node:fs', () => ({
  existsSync: mock(),
  writeFileSync: mock(),
}))
mock.module('@devup-ui/wasm', () => ({
  codeExtract: mock(),
  getDefaultTheme: mock(),
  getThemeInterface: mock(),
  registerTheme: mock(),
  setPrefix: mock(),
}))

describe('DevupUIRsbuildPlugin', () => {
  beforeEach(() => {
    ;(mkdir as Mock<typeof mkdir>).mockReset()
    ;(readFile as Mock<typeof readFile>).mockReset()
    ;(writeFile as Mock<typeof writeFile>).mockReset()
    ;(existsSync as Mock<typeof existsSync>).mockReset()
    ;(writeFileSync as Mock<typeof writeFileSync>).mockReset()
    ;(codeExtract as Mock<typeof codeExtract>).mockReset()
    ;(getDefaultTheme as Mock<typeof getDefaultTheme>).mockReset()
    ;(getThemeInterface as Mock<typeof getThemeInterface>).mockReset()
    ;(registerTheme as Mock<typeof registerTheme>).mockReset()
    ;(setPrefix as Mock<typeof setPrefix>).mockReset()
    ;(mkdir as Mock<typeof mkdir>).mockResolvedValue(undefined)
    ;(writeFile as Mock<typeof writeFile>).mockResolvedValue(undefined)
  })

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
    ;(readFile as Mock<typeof readFile>).mockResolvedValueOnce(
      JSON.stringify({}),
    )
    ;(getThemeInterface as Mock<typeof getThemeInterface>).mockReturnValue(
      'interface code',
    )
    ;(existsSync as Mock<typeof existsSync>).mockImplementation((path) => {
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
    ;(readFile as Mock<typeof readFile>).mockResolvedValueOnce(
      JSON.stringify({}),
    )
    ;(getThemeInterface as Mock<typeof getThemeInterface>).mockReturnValue('')
    ;(existsSync as Mock<typeof existsSync>).mockImplementation((path) => {
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
    expect(writeFileSync).not.toHaveBeenCalled()
  })

  it('should error when write data files', async () => {
    const originalConsoleError = console.error
    console.error = mock()
    ;(readFile as Mock<typeof readFile>).mockRejectedValueOnce('error')
    ;(existsSync as Mock<typeof existsSync>).mockImplementation((path) => {
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
    ;(codeExtract as Mock<typeof codeExtract>).mockReturnValue({
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
    ;(codeExtract as Mock<typeof codeExtract>).mockReturnValue({
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
      expect(writeFile).toHaveBeenCalledWith(
        resolve('df', 'devup-ui', 'devup-ui.css'),
        expect.stringMatching(/\/\* src\/App\.tsx \d+ \*\//),
        'utf-8',
      )
    }
    expect(writeFile).toHaveBeenCalledWith(
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
    ;(writeFile as Mock<typeof writeFile>).mockResolvedValueOnce(undefined)
    ;(readFile as Mock<typeof readFile>).mockResolvedValueOnce(
      JSON.stringify({}),
    )
    ;(getThemeInterface as Mock<typeof getThemeInterface>).mockReturnValue(
      'interface code',
    )
    ;(getDefaultTheme as Mock<typeof getDefaultTheme>).mockReturnValue(
      options.getDefaultTheme,
    )
    ;(existsSync as Mock<typeof existsSync>).mockImplementation((path) => {
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
      expect(readFile).toHaveBeenCalledWith('devup.json', 'utf-8')
      expect(registerTheme).toHaveBeenCalledWith({})
      expect(getThemeInterface).toHaveBeenCalledWith(
        '@devup-ui/react',
        'CustomColors',
        'DevupThemeTypography',
        'DevupTheme',
      )
      expect(writeFile).toHaveBeenCalledWith(
        join('df', 'theme.d.ts'),
        'interface code',
        'utf-8',
      )
    } else {
      expect(registerTheme).toHaveBeenCalledWith({})
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
    expect(setPrefix).toHaveBeenCalledWith('my-prefix')
  })
})
