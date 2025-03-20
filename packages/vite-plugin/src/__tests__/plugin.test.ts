import { existsSync, readFileSync, writeFileSync } from 'node:fs'
import { writeFile } from 'node:fs/promises'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import {
  codeExtract,
  getCss,
  getDefaultTheme,
  getThemeInterface,
} from '@devup-ui/wasm'
import { describe } from 'vitest'

import { DevupUI } from '../plugin'

vi.mock('@devup-ui/wasm')
vi.mock('node:fs')
vi.mock('node:fs/promises')

const _filename = fileURLToPath(import.meta.url)
const _dirname = resolve(dirname(_filename), '..')
beforeEach(() => {
  vi.resetAllMocks()
})

describe('devupUIPlugin', () => {
  console.error = vi.fn()
  it('should write data files', () => {
    const devupPath = 'devup.json'
    const interfacePath = '.df'
    const cssFile = join(_dirname, 'devup-ui.css')
    const libPackage = '@devup-ui/react'
    vi.mocked(existsSync).mockReturnValueOnce(false).mockReturnValueOnce(true)
    vi.mocked(getThemeInterface).mockReturnValue('interface code')
    vi.mocked(readFileSync).mockReturnValueOnce('{"theme": {}}')
    const options = {
      package: libPackage,
      cssFile,
      devupPath,
      interfacePath,
    }
    const plugin = DevupUI(options)
    expect(plugin).toEqual({
      name: 'devup-ui',
      load: expect.any(Function),
      resolveId: expect.any(Function),
      config: expect.any(Function),
      watchChange: expect.any(Function),
      enforce: 'pre',
      transform: expect.any(Function),
      apply: expect.any(Function),
    })
    expect(existsSync).toHaveBeenCalledWith(devupPath)
    expect(getThemeInterface).toHaveBeenCalledWith(
      libPackage,
      'DevupThemeColors',
      'DevupThemeTypography',
      'DevupTheme',
    )
    expect(readFileSync).toHaveBeenCalledWith(devupPath, 'utf-8')
    expect(existsSync).toHaveBeenCalledWith(interfacePath)
    expect((plugin as any).config()).toEqual({
      define: {
        'process.env.DEVUP_UI_DEFAULT_THEME': JSON.stringify(getDefaultTheme()),
      },
      server: {
        watch: {
          ignored: [`!${devupPath}`],
        },
      },
      build: {
        rollupOptions: {
          output: {
            manualChunks: expect.any(Function),
          },
        },
      },
    })
    vi.clearAllMocks()
    vi.mocked(existsSync).mockReturnValue(true)
    vi.mocked(readFileSync).mockReturnValueOnce('{"theme": {}}')
    ;(plugin as any).watchChange(devupPath)
    expect(readFileSync).toBeCalledTimes(1)

    vi.clearAllMocks()
    vi.mocked(existsSync).mockReturnValue(true)
    ;(plugin as any).watchChange('dif')
    expect(readFileSync).toBeCalledTimes(0)

    vi.clearAllMocks()
    vi.mocked(existsSync).mockReturnValue(false)
    ;(plugin as any).watchChange(devupPath)
    expect(readFileSync).toBeCalledTimes(0)

    vi.clearAllMocks()
    vi.mocked(existsSync).mockReturnValue(true)
    vi.mocked(readFileSync).mockImplementation(() => {
      throw new Error('error')
    })
    ;(plugin as any).watchChange(devupPath)
    expect(readFileSync).toBeCalledTimes(1)

    vi.clearAllMocks()
    ;(plugin as any).transform('code', 'file')
    expect(readFileSync).toBeCalledTimes(0)

    vi.clearAllMocks()
    ;(plugin as any).transform('code', 'node_modules')
    expect(readFileSync).toBeCalledTimes(0)

    vi.clearAllMocks()
    ;(plugin as any).transform('code', 'wrong.css')
    expect(readFileSync).toBeCalledTimes(0)

    vi.clearAllMocks()
    vi.mocked(codeExtract).mockReturnValueOnce({
      css: 'css code',
      code: 'code',
    } as any)
    ;(plugin as any).transform('code', 'correct.ts')
    expect(writeFile).toBeCalledTimes(1)

    vi.clearAllMocks()
    vi.mocked(codeExtract).mockReturnValueOnce({
      css: undefined,
      code: 'code',
    } as any)
    ;(plugin as any).transform('code', 'correct.ts')
    expect(writeFile).toBeCalledTimes(0)
    ;(plugin as any).apply({}, { command: 'serve' })
    vi.clearAllMocks()
    vi.mocked(codeExtract).mockReturnValueOnce({
      css: 'css code next',
      code: 'code',
    } as any)
    ;(plugin as any).transform('code', 'correct.ts')
    expect(writeFile).toBeCalledTimes(1)
  })
  it('should transform code', () => {
    const devupPath = 'devup.json'
    const interfacePath = '.df'
    const cssFile = join(_dirname, 'devup-ui.css')
    const libPackage = '@devup-ui/react'
    vi.mocked(existsSync).mockReturnValueOnce(false).mockReturnValueOnce(true)
    vi.mocked(getThemeInterface).mockReturnValue('interface code')
    vi.mocked(readFileSync).mockReturnValueOnce('{"theme": {}}')
    const options = {
      package: libPackage,
      cssFile,
      devupPath,
      interfacePath,
    }
    const plugin = DevupUI(options)
    expect(plugin).toEqual({
      name: 'devup-ui',
      load: expect.any(Function),
      resolveId: expect.any(Function),
      config: expect.any(Function),
      watchChange: expect.any(Function),
      enforce: 'pre',
      transform: expect.any(Function),
      apply: expect.any(Function),
    })
    expect(existsSync).toHaveBeenCalledWith(devupPath)
    expect(getThemeInterface).toHaveBeenCalledWith(
      libPackage,
      'DevupThemeColors',
      'DevupThemeTypography',
      'DevupTheme',
    )
    expect(readFileSync).toHaveBeenCalledWith(devupPath, 'utf-8')
    expect(existsSync).toHaveBeenCalledWith(interfacePath)
    vi.clearAllMocks()
    vi.mocked(codeExtract).mockReturnValueOnce({
      css: 'css code 1223444',
      code: 'code',
    } as any)
    // eslint-disable-next-line prefer-spread
    ;(plugin as any).apply(null, {
      command: 'serve',
    })
    vi.stubEnv('NODE_ENV', 'development')
    ;(plugin as any).transform('code', 'correct.ts')
    expect(writeFile).toBeCalledTimes(1)
  })
  it('should not extract code', () => {
    const devupPath = 'devup.json'
    const interfacePath = '.df'
    const cssFile = join(_dirname, 'devup-ui.css')
    const libPackage = '@devup-ui/react'
    vi.mocked(existsSync).mockReturnValueOnce(false).mockReturnValueOnce(true)
    vi.mocked(getThemeInterface).mockReturnValue('interface code')
    vi.mocked(readFileSync).mockReturnValueOnce('{"theme": {}}')
    const options = {
      package: libPackage,
      cssFile,
      devupPath,
      interfacePath,
      extractCss: false,
    }
    const plugin = DevupUI(options)
    expect(plugin).toEqual({
      name: 'devup-ui',
      load: expect.any(Function),
      resolveId: expect.any(Function),
      config: expect.any(Function),
      watchChange: expect.any(Function),
      enforce: 'pre',
      transform: expect.any(Function),
      apply: expect.any(Function),
    })
    expect(existsSync).toHaveBeenCalledWith(devupPath)
    expect(getThemeInterface).toHaveBeenCalledWith(
      libPackage,
      'DevupThemeColors',
      'DevupThemeTypography',
      'DevupTheme',
    )
    expect(readFileSync).toHaveBeenCalledWith(devupPath, 'utf-8')
    expect(existsSync).toHaveBeenCalledWith(interfacePath)
    vi.clearAllMocks()
    ;(plugin as any).transform('code', 'correct.tsx')
    expect(readFileSync).toBeCalledTimes(0)
  })
  it('should catch error', () => {
    const devupPath = 'devup.json'
    const interfacePath = '.df'
    const cssFile = join(_dirname, 'devup-ui.css')
    const libPackage = '@devup-ui/react'
    vi.mocked(existsSync).mockReturnValueOnce(false).mockReturnValueOnce(true)
    vi.mocked(getThemeInterface).mockReturnValue('interface code')
    vi.mocked(readFileSync).mockReturnValueOnce('{"theme": {}}')
    vi.mocked(writeFileSync).mockImplementation(() => {
      throw new Error('error')
    })
    const options = {
      package: libPackage,
      cssFile,
      devupPath,
      interfacePath,
    }
    const plugin = DevupUI(options)
    expect(plugin).toEqual({
      name: 'devup-ui',
      load: expect.any(Function),
      resolveId: expect.any(Function),
      config: expect.any(Function),
      watchChange: expect.any(Function),
      enforce: 'pre',
      transform: expect.any(Function),
      apply: expect.any(Function),
    })
    expect(existsSync).toHaveBeenCalledWith(devupPath)
    expect(getThemeInterface).toHaveBeenCalledWith(
      libPackage,
      'DevupThemeColors',
      'DevupThemeTypography',
      'DevupTheme',
    )
    expect(readFileSync).toHaveBeenCalledWith(devupPath, 'utf-8')
    expect(existsSync).toHaveBeenCalledWith(interfacePath)
  })

  it('should return true on apply', () => {
    const devupPath = 'devup.json'
    const interfacePath = '.df'
    const cssFile = join(_dirname, 'devup-ui.css')
    const libPackage = '@devup-ui/react'
    vi.mocked(existsSync).mockReturnValueOnce(false).mockReturnValueOnce(true)
    vi.mocked(getThemeInterface).mockReturnValue('interface code')
    vi.mocked(readFileSync).mockReturnValueOnce('{"theme": {}}')
    const options = {
      package: libPackage,
      cssFile,
      devupPath,
      interfacePath,
    }
    const plugin = DevupUI(options)
    expect(plugin).toEqual({
      name: 'devup-ui',
      config: expect.any(Function),
      load: expect.any(Function),
      resolveId: expect.any(Function),
      watchChange: expect.any(Function),
      enforce: 'pre',
      transform: expect.any(Function),
      apply: expect.any(Function),
    })
    expect(existsSync).toHaveBeenCalledWith(devupPath)
    expect(getThemeInterface).toHaveBeenCalledWith(
      libPackage,
      'DevupThemeColors',
      'DevupThemeTypography',
      'DevupTheme',
    )
    expect(readFileSync).toHaveBeenCalledWith(devupPath, 'utf-8')
    expect(existsSync).toHaveBeenCalledWith(interfacePath)
    expect((plugin as any).apply({}, { command: 'build' })).toBe(true)
  })

  describe('basic', () => {
    const devupPath = 'devup.json'
    const interfacePath = '.df'
    const cssFile = join(_dirname, 'devup-ui.css')
    const libPackage = '@devup-ui/react'
    vi.mocked(existsSync).mockReturnValueOnce(false).mockReturnValueOnce(true)
    vi.mocked(getThemeInterface).mockReturnValue('interface code')
    vi.mocked(readFileSync).mockReturnValueOnce('{"theme": {}}')
    const options = {
      package: libPackage,
      cssFile,
      devupPath,
      interfacePath,
    }
    const plugin = DevupUI(options)
    it('should merge chunks', () => {
      expect(
        (plugin as any)
          .config()
          .build.rollupOptions.output.manualChunks('code', 'code'),
      ).toBeUndefined()

      expect(
        (plugin as any)
          .config()
          .build.rollupOptions.output.manualChunks('devup-ui.css', 'code'),
      ).toEqual('devup-ui.css')
      expect(
        (plugin as any)
          .config()
          .build.rollupOptions.output.manualChunks('devup-ui.css?v=1', 'code'),
      ).toEqual('devup-ui.css')

      const plugin1 = DevupUI({
        package: libPackage,
        cssFile,
        devupPath,
        interfacePath,
        extractCss: false,
      })
      expect((plugin1 as any).config().build).toBeUndefined()
    })
    it('should resolveId', () => {
      expect((plugin as any).resolveId('code', 'code')).toBeUndefined()
      expect(
        (plugin as any)
          .resolveId(cssFile, 'code')
          .startsWith('devup-ui.css?t='),
      ).toBe(true)
    })
    it('should load', () => {
      Date.now = () => 1
      expect((plugin as any).load('code')).toBeUndefined()
      expect((plugin as any).load(cssFile)).toBeUndefined()
      vi.mocked(getCss).mockReturnValueOnce('css code')
      expect(
        (plugin as any).load('devup-ui.css?v=some').length.toString(),
      ).toBe('css code'.length.toString())
    })
  })
})
