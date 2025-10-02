import { existsSync } from 'node:fs'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import {
  codeExtract,
  getCss,
  getDefaultTheme,
  getThemeInterface,
  registerTheme,
  setDebug,
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

describe('devupUIVitePlugin', () => {
  console.error = vi.fn()
  it('should apply default options', () => {
    const plugin = DevupUI({})
    expect(plugin).toEqual({
      name: 'devup-ui',
      config: expect.any(Function),
      load: expect.any(Function),
      watchChange: expect.any(Function),
      enforce: 'pre',
      transform: expect.any(Function),
      apply: expect.any(Function),
      generateBundle: expect.any(Function),
      configResolved: expect.any(Function),
      resolveId: expect.any(Function),
    })
    expect((plugin as any).apply()).toBe(true)
  })
  it.each(
    globalThis.createTestMatrix({
      debug: [true, false],
      extractCss: [true, false],
    }),
  )('should apply options', async (options) => {
    const plugin = DevupUI(options)
    expect(setDebug).toHaveBeenCalledWith(options.debug)
    if (options.extractCss) {
      expect(
        (plugin as any)
          .config()
          .build.rollupOptions.output.manualChunks('devup-ui.css', 'code'),
      ).toEqual('devup-ui.css')
    } else {
      expect((plugin as any).config().build).toBeUndefined()
    }
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
    vi.mocked(writeFile).mockResolvedValueOnce(undefined)
    vi.mocked(readFile).mockResolvedValueOnce(JSON.stringify({}))
    vi.mocked(getThemeInterface).mockReturnValue('interface code')
    vi.mocked(getDefaultTheme).mockReturnValue(options.getDefaultTheme)
    vi.mocked(existsSync).mockImplementation((path) => {
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
    await (plugin as any).configResolved()
    if (options.existsDevupFile) {
      expect(readFile).toHaveBeenCalledWith('devup.json', 'utf-8')
      expect(registerTheme).toHaveBeenCalledWith({})
      expect(getThemeInterface).toHaveBeenCalledWith(
        '@devup-ui/react',
        'DevupThemeColors',
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

    const config = (plugin as any).config()
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
    vi.mocked(writeFile).mockResolvedValueOnce(undefined)
    vi.mocked(getThemeInterface).mockReturnValue('interface code')
    vi.mocked(existsSync).mockReturnValue(true)
    vi.mocked(readFile).mockImplementation(() => {
      throw new Error('error')
    })
    const plugin = DevupUI({})
    await (plugin as any).configResolved()
    expect(registerTheme).toHaveBeenCalledWith({})
    expect(writeFile).toHaveBeenCalledWith(
      join('df', '.gitignore'),
      '*',
      'utf-8',
    )
  })

  it('should watch change', async () => {
    vi.mocked(writeFile).mockResolvedValueOnce(undefined)
    vi.mocked(getThemeInterface).mockReturnValue('interface code')
    vi.mocked(existsSync).mockReturnValue(true)
    vi.mocked(readFile).mockResolvedValueOnce(
      JSON.stringify({ theme: 'theme' }),
    )
    const plugin = DevupUI({})
    await (plugin as any).watchChange('devup.json')
    expect(writeFile).toHaveBeenCalledWith(
      join('df', 'theme.d.ts'),
      'interface code',
      'utf-8',
    )

    await (plugin as any).watchChange('wrong')
  })

  it('should print error when watch change error', async () => {
    vi.mocked(writeFile).mockResolvedValueOnce(undefined)
    vi.mocked(getThemeInterface).mockReturnValue('interface code')
    vi.mocked(existsSync).mockReturnValueOnce(true).mockReturnValueOnce(false)
    vi.mocked(mkdir).mockImplementation(() => {
      throw new Error('error')
    })
    const plugin = DevupUI({})
    await (plugin as any).watchChange('devup.json')
    expect(console.error).toHaveBeenCalledWith(expect.any(Error))
  })

  it('should load', () => {
    vi.mocked(getCss).mockReturnValue('css code')
    const plugin = DevupUI({})
    expect((plugin as any).load('devup-ui.css')).toEqual(expect.any(String))
    expect((plugin as any).load('devup-ui-10.css')).toEqual(expect.any(String))
  })

  it.each(
    createTestMatrix({
      extractCss: [true, false],
      updatedBaseStyle: [true, false],
    }),
  )('should transform', async (options) => {
    vi.mocked(getCss).mockReturnValue('css code')
    vi.mocked(codeExtract).mockReturnValue({
      css: 'css code',
      code: 'code',
      cssFile: 'devup-ui.css',
      map: undefined,
      updatedBaseStyle: options.updatedBaseStyle,
      free: vi.fn(),
      [Symbol.dispose]: vi.fn(),
    })

    const plugin = DevupUI(options)

    expect(await (plugin as any).transform('code', 'devup-ui.wrong')).toEqual(
      undefined,
    )
    expect(await (plugin as any).transform('code', 'devup-ui.tsx')).toEqual(
      options.extractCss ? { code: 'code' } : undefined,
    )

    if (options.extractCss) {
      expect(
        await (plugin as any).transform('code', 'node_modules/test/index.tsx'),
      ).toEqual(undefined)
      expect(
        await (plugin as any).transform(
          'code',
          'node_modules/@devup-ui/hello/index.tsx',
        ),
      ).toEqual({ code: 'code' })

      vi.mocked(codeExtract).mockReturnValue({
        css: 'css code test next',
        code: 'code',
        cssFile: 'devup-ui.css',
        map: undefined,
        updatedBaseStyle: options.updatedBaseStyle,
        free: vi.fn(),
        [Symbol.dispose]: vi.fn(),
      })
      expect(writeFile).toHaveBeenCalledWith(
        join(resolve('df', 'devup-ui'), 'devup-ui.css'),
        expect.stringMatching(
          /\/\* node_modules[/\\]@devup-ui[/\\]hello[/\\]index\.tsx \d+ \*\//,
        ),
        'utf-8',
      )
      expect(
        await (plugin as any).transform(
          'code',
          'node_modules/@devup-ui/hello/index.tsx',
        ),
      ).toEqual({ code: 'code' })
    }
    expect(await (plugin as any).load('devup-ui.css')).toEqual(
      expect.any(String),
    )

    vi.mocked(codeExtract).mockReturnValue({
      css: 'long css code',
      code: 'long code',
      cssFile: 'devup-ui.css',
      map: undefined,
      updatedBaseStyle: options.updatedBaseStyle,
      free: vi.fn(),
      [Symbol.dispose]: vi.fn(),
    })
    expect(await (plugin as any).transform('code', 'devup-ui.tsx')).toEqual(
      options.extractCss ? { code: 'long code' } : undefined,
    )
  })

  it.each(
    createTestMatrix({
      extractCss: [true, false],
    }),
  )('should generateBundle', async (options) => {
    vi.mocked(getCss).mockReturnValue('css code test')
    const plugin = DevupUI({ extractCss: options.extractCss, singleCss: true })
    const bundle = {
      'devup-ui.css': { source: 'css code', name: 'devup-ui.css' },
    } as any
    ;(plugin as any).load('devup-ui.css')
    await (plugin as any).generateBundle({}, bundle)
    if (options.extractCss) {
      expect(bundle['devup-ui.css'].source).toEqual('css code test')
    } else {
      expect(bundle['devup-ui.css'].source).toEqual('css code')
    }
  })
  it('should resolveId', () => {
    vi.mocked(getCss).mockReturnValue('css code')
    {
      const plugin = DevupUI({})
      expect(
        (plugin as any).resolveId('devup-ui.css', 'df/devup-ui/devup-ui.css'),
      ).toEqual(expect.any(String))
    }

    {
      const plugin = DevupUI({
        cssDir: '',
      })
      expect((plugin as any).resolveId('devup-ui.css')).toEqual(
        expect.any(String),
      )
    }
  })
  it('should resolve id with cssMap', () => {
    vi.mocked(getCss).mockReturnValue('css code')
    const plugin = DevupUI({})
    ;(plugin as any).load('devup-ui.css')
    expect(
      (plugin as any).resolveId('devup-ui.css', 'df/devup-ui/devup-ui.css'),
    ).toEqual(expect.any(String))
  })
})
