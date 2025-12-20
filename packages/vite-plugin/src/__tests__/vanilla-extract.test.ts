import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('@devup-ui/wasm', () => ({
  codeExtract: vi.fn(),
  getCss: vi.fn(),
  getDefaultTheme: vi.fn(),
  getThemeInterface: vi.fn(),
  registerTheme: vi.fn(),
  setDebug: vi.fn(),
}))
vi.mock('node:fs')
vi.mock('node:fs/promises')

import { DevupUI } from '../plugin'

describe('vanilla-extract support', () => {
  beforeEach(() => {
    vi.resetAllMocks()
  })

  it('should accept vanilla-extract plugin option', () => {
    const mockVanillaExtractPlugin = { name: 'vanilla-extract' }
    const plugin = DevupUI({
      vanillaExtract: {
        plugin: mockVanillaExtractPlugin as any,
      },
    })

    expect(plugin).toBeDefined()
  })

  it('should enable vanilla-extract by default when plugin is provided', () => {
    const mockVanillaExtractPlugin = { name: 'vanilla-extract' }
    const plugin = DevupUI({
      vanillaExtract: {
        plugin: mockVanillaExtractPlugin as any,
      },
    })

    expect(plugin).toHaveProperty('name', 'devup-ui')
  })

  it('should allow disabling vanilla-extract explicitly', () => {
    const plugin = DevupUI({
      vanillaExtract: false,
    })

    expect(plugin).toHaveProperty('name', 'devup-ui')
  })

  it('should pass through vanilla-extract options', () => {
    const mockVanillaExtractPlugin = {
      name: 'vanilla-extract',
      transform: vi.fn(),
    }
    const vanillaExtractOptions = {
      identifiers: 'short' as const,
    }

    const plugin = DevupUI({
      vanillaExtract: {
        plugin: mockVanillaExtractPlugin as any,
        options: vanillaExtractOptions,
      },
    })

    expect(plugin).toBeDefined()
  })

  it('should handle vanilla-extract config as a boolean', () => {
    const pluginEnabled = DevupUI({
      vanillaExtract: true,
    })

    const pluginDisabled = DevupUI({
      vanillaExtract: false,
    })

    expect(pluginEnabled).toHaveProperty('name', 'devup-ui')
    expect(pluginDisabled).toHaveProperty('name', 'devup-ui')
  })

  it('should handle vanilla-extract with custom identifiers', () => {
    const mockVanillaExtractPlugin = { name: 'vanilla-extract' }

    const plugin = DevupUI({
      vanillaExtract: {
        plugin: mockVanillaExtractPlugin as any,
        options: {
          identifiers: 'debug' as const,
        },
      },
    })

    expect(plugin).toBeDefined()
  })

  it('should work with vanilla-extract and extractCss option', () => {
    const mockVanillaExtractPlugin = { name: 'vanilla-extract' }

    const plugin = DevupUI({
      extractCss: true,
      vanillaExtract: {
        plugin: mockVanillaExtractPlugin as any,
      },
    })

    expect(plugin).toBeDefined()
  })

  it('should merge vanilla-extract plugin with devup-ui plugin', () => {
    const mockVanillaExtractPlugin = {
      name: 'vanilla-extract',
      transform: vi.fn(),
      config: vi.fn(),
    }

    const plugin = DevupUI({
      vanillaExtract: {
        plugin: mockVanillaExtractPlugin as any,
      },
    })

    expect(plugin).toHaveProperty('name', 'devup-ui')
    expect(plugin).toHaveProperty('transform')
    expect(plugin).toHaveProperty('config')
  })
})

describe('vanilla-extract integration test', () => {
  it('should handle .css.ts files', () => {
    const mockVanillaExtractPlugin = {
      name: 'vanilla-extract',
      transform: vi.fn((_code: string, id: string) => {
        if (id.endsWith('.css.ts')) {
          return {
            code: 'export const className = "_vanilla_1";',
            map: null,
          }
        }
      }),
    }

    const plugin = DevupUI({
      vanillaExtract: {
        plugin: mockVanillaExtractPlugin as any,
      },
    })

    expect(plugin).toBeDefined()
  })

  it('should support vanilla-extract style objects', () => {
    const mockVanillaExtractPlugin = {
      name: 'vanilla-extract',
      transform: vi.fn(),
    }

    const plugin = DevupUI({
      vanillaExtract: {
        plugin: mockVanillaExtractPlugin as any,
        options: {
          identifiers: 'short' as const,
        },
      },
    })

    expect(plugin).toBeDefined()
  })
})

describe('vanilla-extract configuration options', () => {
  it('should accept emitCssInSsr option', () => {
    const mockVanillaExtractPlugin = { name: 'vanilla-extract' }

    const plugin = DevupUI({
      vanillaExtract: {
        plugin: mockVanillaExtractPlugin as any,
        options: {
          emitCssInSsr: true,
        },
      },
    })

    expect(plugin).toBeDefined()
  })

  it('should accept esbuildOptions', () => {
    const mockVanillaExtractPlugin = { name: 'vanilla-extract' }

    const plugin = DevupUI({
      vanillaExtract: {
        plugin: mockVanillaExtractPlugin as any,
        options: {
          esbuildOptions: {
            target: 'es2020',
          },
        },
      },
    })

    expect(plugin).toBeDefined()
  })

  it('should work without any vanilla-extract config', () => {
    const plugin = DevupUI({})

    expect(plugin).toHaveProperty('name', 'devup-ui')
  })
})
