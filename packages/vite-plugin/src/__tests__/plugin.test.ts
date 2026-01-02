import * as nodePath from 'node:path'
import { fileURLToPath } from 'node:url'

import { beforeEach, describe, expect, it, mock } from 'bun:test'

const { dirname, join, resolve, relative: originalRelative } = nodePath

const mockExistsSync = mock(() => false)
const mockMkdir = mock(() => Promise.resolve())
const mockReadFile = mock(() => Promise.resolve('{}'))
const mockWriteFile = mock(() => Promise.resolve())
const mockRelative = mock((from: string, to: string) =>
  originalRelative(from, to),
)

mock.module('node:fs', () => ({
  existsSync: mockExistsSync,
}))

mock.module('node:fs/promises', () => ({
  mkdir: mockMkdir,
  readFile: mockReadFile,
  writeFile: mockWriteFile,
}))

mock.module('node:path', () => ({
  ...nodePath,
  relative: mockRelative,
}))

const mockCodeExtract = mock(() => ({
  css: 'css code',
  code: 'code',
  cssFile: 'devup-ui.css',
  map: undefined,
  updatedBaseStyle: false,
  free: mock(),
  [Symbol.dispose]: mock(),
}))
const mockGetCss = mock(() => 'css code')
const mockGetDefaultTheme = mock(() => 'default')
const mockGetThemeInterface = mock(() => 'interface code')
const mockRegisterTheme = mock()
const mockSetDebug = mock()
const mockSetPrefix = mock()

mock.module('@devup-ui/wasm', () => ({
  codeExtract: mockCodeExtract,
  getCss: mockGetCss,
  getDefaultTheme: mockGetDefaultTheme,
  getThemeInterface: mockGetThemeInterface,
  registerTheme: mockRegisterTheme,
  setDebug: mockSetDebug,
  setPrefix: mockSetPrefix,
}))

import { DevupUI } from '../plugin'

const _filename = fileURLToPath(import.meta.url)
const _dirname = resolve(dirname(_filename), '..')

beforeEach(() => {
  mockExistsSync.mockReset()
  mockMkdir.mockReset()
  mockReadFile.mockReset()
  mockWriteFile.mockReset()
  mockRelative.mockReset()
  mockCodeExtract.mockReset()
  mockGetCss.mockReset()
  mockGetDefaultTheme.mockReset()
  mockGetThemeInterface.mockReset()
  mockRegisterTheme.mockReset()
  mockSetDebug.mockReset()
  mockSetPrefix.mockReset()

  mockExistsSync.mockReturnValue(false)
  mockMkdir.mockResolvedValue(undefined)
  mockReadFile.mockResolvedValue('{}')
  mockWriteFile.mockResolvedValue(undefined)
  mockGetCss.mockReturnValue('css code')
  mockGetDefaultTheme.mockReturnValue('default')
  mockGetThemeInterface.mockReturnValue('interface code')
  mockCodeExtract.mockReturnValue({
    css: 'css code',
    code: 'code',
    cssFile: 'devup-ui.css',
    map: undefined,
    updatedBaseStyle: false,
    free: mock(),
    [Symbol.dispose]: mock(),
  })
  mockRelative.mockImplementation((from: string, to: string) =>
    originalRelative(from, to),
  )
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
    expect(mockSetDebug).toHaveBeenCalledWith(options.debug)
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
    mockWriteFile.mockResolvedValueOnce(undefined)
    mockReadFile.mockResolvedValueOnce(JSON.stringify({}))
    mockGetThemeInterface.mockReturnValue('interface code')
    mockGetDefaultTheme.mockReturnValue(options.getDefaultTheme)
    mockExistsSync.mockImplementation((path) => {
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
      expect(mockReadFile).toHaveBeenCalledWith('devup.json', 'utf-8')
      expect(mockRegisterTheme).toHaveBeenCalledWith({})
      expect(mockGetThemeInterface).toHaveBeenCalledWith(
        '@devup-ui/react',
        'CustomColors',
        'DevupThemeTypography',
        'DevupTheme',
      )
      expect(mockWriteFile).toHaveBeenCalledWith(
        join('df', 'theme.d.ts'),
        'interface code',
        'utf-8',
      )
    } else {
      expect(mockRegisterTheme).toHaveBeenCalledWith({})
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
    mockWriteFile.mockResolvedValueOnce(undefined)
    mockGetThemeInterface.mockReturnValue('interface code')
    mockExistsSync.mockReturnValue(true)
    mockReadFile.mockImplementation(() => {
      throw new Error('error')
    })
    const plugin = DevupUI({})
    await (plugin as any).configResolved()
    expect(mockRegisterTheme).toHaveBeenCalledWith({})
    expect(mockWriteFile).toHaveBeenCalledWith(
      join('df', '.gitignore'),
      '*',
      'utf-8',
    )
  })

  it('should watch change', async () => {
    mockWriteFile.mockResolvedValueOnce(undefined)
    mockGetThemeInterface.mockReturnValue('interface code')
    mockExistsSync.mockReturnValue(true)
    mockReadFile.mockResolvedValueOnce(JSON.stringify({ theme: 'theme' }))
    const plugin = DevupUI({})
    await (plugin as any).watchChange('devup.json')
    expect(mockWriteFile).toHaveBeenCalledWith(
      join('df', 'theme.d.ts'),
      'interface code',
      'utf-8',
    )

    await (plugin as any).watchChange('wrong')
  })

  it('should print error when watch change error', async () => {
    mockWriteFile.mockResolvedValueOnce(undefined)
    mockGetThemeInterface.mockReturnValue('interface code')
    mockExistsSync.mockReturnValueOnce(true).mockReturnValueOnce(false)
    mockMkdir.mockImplementation(() => {
      throw new Error('error')
    })
    const plugin = DevupUI({})
    await (plugin as any).watchChange('devup.json')
    expect(console.error).toHaveBeenCalledWith(expect.any(Error))
  })

  it('should load', () => {
    mockGetCss.mockReturnValue('css code')
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
    mockGetCss.mockReturnValue('css code')
    mockCodeExtract.mockReturnValue({
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

      mockCodeExtract.mockReturnValue({
        css: 'css code test next',
        code: 'code',
        cssFile: 'devup-ui.css',
        map: undefined,
        updatedBaseStyle: options.updatedBaseStyle,
        free: mock(),
        [Symbol.dispose]: mock(),
      })
      expect(mockWriteFile).toHaveBeenCalledWith(
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

    mockCodeExtract.mockReturnValue({
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
    mockGetCss.mockReturnValue('css code test')
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
    mockGetCss.mockReturnValue('css code')
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
    mockGetCss.mockReturnValue('css code')
    const plugin = DevupUI({})
    expect((plugin as any).load('devup-ui.css')).toEqual(expect.any(String))
    expect((plugin as any).load('other.css')).toEqual(undefined)

    expect(
      (plugin as any).resolveId('devup-ui.css', 'df/devup-ui/devup-ui.css'),
    ).toEqual(expect.any(String))
  })

  it('should not write interface code when no theme', async () => {
    mockReadFile.mockResolvedValueOnce(JSON.stringify({}))
    mockGetThemeInterface.mockReturnValue('')
    mockExistsSync.mockReturnValue(true)
    const plugin = DevupUI({})
    await (plugin as any).configResolved()
    expect(mockWriteFile).not.toHaveBeenCalledWith(
      join('df', 'theme.d.ts'),
      expect.any(String),
      'utf-8',
    )
  })

  it('sholud add relative path to css file', async () => {
    mockGetCss.mockReturnValue('css code')
    mockCodeExtract.mockReturnValue({
      css: 'css code',
      code: 'code',
      cssFile: 'devup-ui.css',
      map: undefined,
      updatedBaseStyle: false,
      free: mock(),
      [Symbol.dispose]: mock(),
    })
    const plugin = DevupUI({})
    mockRelative.mockReturnValue('./df/devup-ui/devup-ui.css')
    await (plugin as any).transform('code', 'foo.tsx')

    expect(mockCodeExtract).toHaveBeenCalledWith(
      'foo.tsx',
      'code',
      '@devup-ui/react',
      './df/devup-ui/devup-ui.css',
      false,
      true,
      false,
    )

    mockRelative.mockReturnValue('df/devup-ui/devup-ui.css')
    await (plugin as any).transform('code', 'foo.tsx')
    expect(mockCodeExtract).toHaveBeenCalledWith(
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
    mockGetCss.mockReturnValue('css code')
    mockCodeExtract.mockReturnValue({
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
    expect(mockWriteFile).not.toHaveBeenCalled()
  })

  it('should not generate bundle when css file is not found', async () => {
    const plugin = DevupUI({})
    const bundle = {} as any
    await (plugin as any).generateBundle({}, bundle)
    expect(bundle).toEqual({})
  })

  it('should call setPrefix when prefix option is provided', () => {
    DevupUI({ prefix: 'my-prefix' })
    expect(mockSetPrefix).toHaveBeenCalledWith('my-prefix')
  })
})
