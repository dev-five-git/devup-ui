import { existsSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import { codeExtract, getThemeInterface } from '@devup-ui/wasm'
import { expect } from 'vitest'

import { DevupUI } from '../plugin'

vi.mock('@devup-ui/wasm')
vi.mock('node:fs')

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
    vi.mocked(existsSync).mockReturnValueOnce(true).mockReturnValueOnce(false)
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
      server: {
        watch: {
          ignored: [`!${devupPath}`],
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
    expect(writeFileSync).toBeCalledTimes(1)

    vi.clearAllMocks()
    vi.mocked(codeExtract).mockReturnValueOnce({
      css: undefined,
      code: 'code',
    } as any)
    ;(plugin as any).transform('code', 'correct.ts')
    expect(writeFileSync).toBeCalledTimes(0)
    ;(plugin as any).apply({}, { command: 'serve' })
    vi.clearAllMocks()
    vi.mocked(codeExtract).mockReturnValueOnce({
      css: 'css code',
      code: 'code',
    } as any)
    ;(plugin as any).transform('code', 'correct.ts')
    expect(writeFileSync).toBeCalledTimes(1)
  })
  it('should not extract code', () => {
    const devupPath = 'devup.json'
    const interfacePath = '.df'
    const cssFile = join(_dirname, 'devup-ui.css')
    const libPackage = '@devup-ui/react'
    vi.mocked(existsSync).mockReturnValueOnce(true).mockReturnValueOnce(false)
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
    vi.mocked(existsSync).mockReturnValueOnce(true).mockReturnValueOnce(false)
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
    vi.mocked(existsSync).mockReturnValueOnce(true).mockReturnValueOnce(false)
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
})
