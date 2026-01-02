import * as fs from 'node:fs'
import * as fsPromises from 'node:fs/promises'
import * as nodePath from 'node:path'

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

const { join, resolve, relative: originalRelative } = nodePath

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

beforeEach(() => {
  existsSyncSpy = spyOn(fs, 'existsSync').mockReturnValue(false)
  mkdirSpy = spyOn(fsPromises, 'mkdir').mockResolvedValue(undefined)
  readFileSpy = spyOn(fsPromises, 'readFile').mockResolvedValue('{}')
  writeFileSpy = spyOn(fsPromises, 'writeFile').mockResolvedValue(undefined)
  relativeSpy = spyOn(nodePath, 'relative').mockImplementation(
    (from: string, to: string) => originalRelative(from, to),
  )
  codeExtractSpy = spyOn(wasm, 'codeExtract').mockReturnValue({
    css: 'css code',
    code: 'code',
    cssFile: 'devup-ui.css',
    map: undefined,
    updatedBaseStyle: false,
    free: mock(),
    [Symbol.dispose]: mock(),
  } as any)
  getCssSpy = spyOn(wasm, 'getCss').mockReturnValue('css code')
  getDefaultThemeSpy = spyOn(wasm, 'getDefaultTheme').mockReturnValue('default')
  getThemeInterfaceSpy = spyOn(wasm, 'getThemeInterface').mockReturnValue(
    'interface code',
  )
  registerThemeSpy = spyOn(wasm, 'registerTheme').mockReturnValue(undefined)
  setDebugSpy = spyOn(wasm, 'setDebug').mockReturnValue(undefined)
  setPrefixSpy = spyOn(wasm, 'setPrefix').mockReturnValue(undefined)
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
})

describe('devupUIVitePlugin', () => {
  console.error = mock()

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
    expect(setDebugSpy).toHaveBeenCalledWith(options.debug)
    if (options.extractCss) {
      expect(
        (plugin as any)
          .config()
          .build.rollupOptions.output.manualChunks('devup-ui.css', 'code'),
      ).toEqual('devup-ui.css')

      expect(
        (plugin as any)
          .config()
          .build.rollupOptions.output.manualChunks('other.css', 'code'),
      ).toEqual(undefined)
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
    await (plugin as any).configResolved()
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
    writeFileSpy.mockResolvedValueOnce(undefined)
    getThemeInterfaceSpy.mockReturnValue('interface code')
    existsSyncSpy.mockReturnValue(true)
    readFileSpy.mockImplementation(() => {
      throw new Error('error')
    })
    const plugin = DevupUI({})
    await (plugin as any).configResolved()
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
    const plugin = DevupUI({})
    await (plugin as any).watchChange('devup.json')
    expect(writeFileSpy).toHaveBeenCalledWith(
      join('df', 'theme.d.ts'),
      'interface code',
      'utf-8',
    )

    await (plugin as any).watchChange('wrong')
  })

  it('should print error when watch change error', async () => {
    writeFileSpy.mockResolvedValueOnce(undefined)
    getThemeInterfaceSpy.mockReturnValue('interface code')
    existsSyncSpy.mockReturnValueOnce(true).mockReturnValueOnce(false)
    mkdirSpy.mockImplementation(() => {
      throw new Error('error')
    })
    const plugin = DevupUI({})
    await (plugin as any).watchChange('devup.json')
    expect(console.error).toHaveBeenCalledWith(expect.any(Error))
  })

  it('should load', () => {
    getCssSpy.mockReturnValue('css code')
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
    getCssSpy.mockReturnValue('css code')
    codeExtractSpy.mockReturnValue({
      css: 'css code',
      code: 'code',
      cssFile: 'devup-ui.css',
      map: undefined,
      updatedBaseStyle: options.updatedBaseStyle,
      free: mock(),
      [Symbol.dispose]: mock(),
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

      codeExtractSpy.mockReturnValue({
        css: 'css code test next',
        code: 'code',
        cssFile: 'devup-ui.css',
        map: undefined,
        updatedBaseStyle: options.updatedBaseStyle,
        free: mock(),
        [Symbol.dispose]: mock(),
      })
      expect(writeFileSpy).toHaveBeenCalledWith(
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

    codeExtractSpy.mockReturnValue({
      css: 'long css code',
      code: 'long code',
      cssFile: 'devup-ui.css',
      map: undefined,
      updatedBaseStyle: options.updatedBaseStyle,
      free: mock(),
      [Symbol.dispose]: mock(),
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
    getCssSpy.mockReturnValue('css code test')
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
    getCssSpy.mockReturnValue('css code')
    {
      const plugin = DevupUI({})
      expect(
        (plugin as any).resolveId('devup-ui.css', 'df/devup-ui/devup-ui.css'),
      ).toEqual(expect.any(String))

      expect(
        (plugin as any).resolveId('other.css', 'df/devup-ui/devup-ui.css'),
      ).toEqual(undefined)
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
    getCssSpy.mockReturnValue('css code')
    const plugin = DevupUI({})
    expect((plugin as any).load('devup-ui.css')).toEqual(expect.any(String))
    expect((plugin as any).load('other.css')).toEqual(undefined)

    expect(
      (plugin as any).resolveId('devup-ui.css', 'df/devup-ui/devup-ui.css'),
    ).toEqual(expect.any(String))
  })

  it('should not write interface code when no theme', async () => {
    readFileSpy.mockResolvedValueOnce(JSON.stringify({}))
    getThemeInterfaceSpy.mockReturnValue('')
    existsSyncSpy.mockReturnValue(true)
    const plugin = DevupUI({})
    await (plugin as any).configResolved()
    expect(writeFileSpy).not.toHaveBeenCalledWith(
      join('df', 'theme.d.ts'),
      expect.any(String),
      'utf-8',
    )
  })

  it('sholud add relative path to css file', async () => {
    getCssSpy.mockReturnValue('css code')
    codeExtractSpy.mockReturnValue({
      css: 'css code',
      code: 'code',
      cssFile: 'devup-ui.css',
      map: undefined,
      updatedBaseStyle: false,
      free: mock(),
      [Symbol.dispose]: mock(),
    })
    const plugin = DevupUI({})
    relativeSpy.mockReturnValue('./df/devup-ui/devup-ui.css')
    await (plugin as any).transform('code', 'foo.tsx')

    expect(codeExtractSpy).toHaveBeenCalledWith(
      'foo.tsx',
      'code',
      '@devup-ui/react',
      './df/devup-ui/devup-ui.css',
      false,
      true,
      false,
    )

    relativeSpy.mockReturnValue('df/devup-ui/devup-ui.css')
    await (plugin as any).transform('code', 'foo.tsx')
    expect(codeExtractSpy).toHaveBeenCalledWith(
      'foo.tsx',
      'code',
      '@devup-ui/react',
      './df/devup-ui/devup-ui.css',
      false,
      true,
      false,
    )
  })

  it('should not create css file when cssFile is empty', async () => {
    getCssSpy.mockReturnValue('css code')
    codeExtractSpy.mockReturnValue({
      css: 'css code',
      code: 'code',
      cssFile: '',
      map: undefined,
      updatedBaseStyle: false,
      free: mock(),
      [Symbol.dispose]: mock(),
    })
    const plugin = DevupUI({})
    await (plugin as any).transform('code', 'foo.tsx')
    expect(writeFileSpy).not.toHaveBeenCalled()
  })

  it('should not generate bundle when css file is not found', async () => {
    const plugin = DevupUI({})
    const bundle = {} as any
    await (plugin as any).generateBundle({}, bundle)
    expect(bundle).toEqual({})
  })

  it('should call setPrefix when prefix option is provided', () => {
    DevupUI({ prefix: 'my-prefix' })
    expect(setPrefixSpy).toHaveBeenCalledWith('my-prefix')
  })
})
