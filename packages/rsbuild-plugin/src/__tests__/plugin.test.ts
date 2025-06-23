import { mkdir, writeFile } from 'node:fs/promises'
import { resolve } from 'node:path'

import { codeExtract } from '@devup-ui/wasm'
import { vi } from 'vitest'

import { DevupUIRsbuildPlugin } from '../plugin'

// Mock dependencies
vi.mock('node:fs/promises')
vi.mock('@devup-ui/wasm', () => ({
  codeExtract: vi.fn().mockReturnValue({
    code: '',
    css: '',
  }),
}))

describe('DevupUIRsbuildPlugin', () => {
  beforeEach(() => {
    vi.resetAllMocks()
    vi.mocked(mkdir).mockResolvedValue(undefined)
    vi.mocked(writeFile).mockResolvedValue(undefined)
  })

  it('should export DevupUIRsbuildPlugin', () => {
    expect(DevupUIRsbuildPlugin).toBeDefined()
  })

  it('should be a function', () => {
    expect(DevupUIRsbuildPlugin).toBeInstanceOf(Function)
  })

  it('should return a plugin object with correct name', async () => {
    const plugin = DevupUIRsbuildPlugin()
    expect(plugin).toBeDefined()
    expect(plugin.name).toBe('devup-ui-rsbuild-plugin')
    expect(typeof plugin.setup).toBe('function')

    const transform = vi.fn()
    await plugin.setup({
      transform,
    } as any)
    expect(transform).toHaveBeenCalled()
  })

  it('should not register css transform', async () => {
    const plugin = DevupUIRsbuildPlugin({
      extractCss: false,
    })
    expect(plugin).toBeDefined()
    expect(plugin.setup).toBeDefined()
    const transform = vi.fn()
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

    const plugin = DevupUIRsbuildPlugin(customOptions)
    expect(plugin).toBeDefined()
    expect(plugin.name).toBe('devup-ui-rsbuild-plugin')
  })
  it('should transform css', async () => {
    const plugin = DevupUIRsbuildPlugin()
    expect(plugin).toBeDefined()
    expect(plugin.setup).toBeDefined()
    const transform = vi.fn()
    await plugin.setup({
      transform,
    } as any)
    expect(transform).toHaveBeenCalled()
    expect(transform).toHaveBeenCalledWith(
      {
        test: resolve('.df', 'devup-ui.css'),
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
    const plugin = DevupUIRsbuildPlugin()
    expect(plugin).toBeDefined()
    expect(plugin.setup).toBeDefined()
    const transform = vi.fn()
    await plugin.setup({
      transform,
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

    vi.mocked(codeExtract).mockReturnValue({
      code: '<div></div>',
      css: '',
    } as any)
    await expect(
      transform.mock.calls[1][1]({
        code: `import { Box } from '@devup-ui/react'
const App = () => <Box></Box>`,
        resourcePath: 'src/App.tsx',
      }),
    ).resolves.toBe('<div></div>')
  })
  it('should transform with include', async () => {
    const plugin = DevupUIRsbuildPlugin({
      include: ['lib'],
    })
    expect(plugin).toBeDefined()
    expect(plugin.setup).toBeDefined()
    const transform = vi.fn()
    await plugin.setup({
      transform,
    } as any)
    expect(transform).toHaveBeenCalled()
    expect(transform).toHaveBeenCalledWith(
      {
        test: /\.(tsx|ts|js|mjs|jsx)$/,
      },
      expect.any(Function),
    )
    vi.mocked(codeExtract).mockReturnValue({
      code: '<div></div>',
      css: '.devup-ui-1 { color: red; }',
    } as any)
    const ret = await transform.mock.calls[1][1]({
      code: `import { Box } from '@devup-ui/react'
const App = () => <Box></Box>`,
      resourcePath: 'src/App.tsx',
    })
    expect(ret).toBe('<div></div>')
    expect(writeFile).toHaveBeenCalledWith(
      resolve('.df', 'devup-ui.css'),
      expect.stringMatching(/\/\* src\/App\.tsx \d+ \*\//),
      {
        encoding: 'utf-8',
      },
    )

    const ret1 = await transform.mock.calls[1][1]({
      code: `import { Box } from '@devup-ui/react'
const App = () => <Box></Box>`,
      resourcePath: 'node_modules/@devup-ui/react/index.tsx',
    })
    expect(ret1).toBe(`import { Box } from '@devup-ui/react'
const App = () => <Box></Box>`)
  })
})
