import * as fsPromises from 'node:fs/promises'
import * as nodePath from 'node:path'
import { join } from 'node:path'

import { type DevupCompilerOutput, loadCompiler } from '@devup-ui/plugin-utils'
import {
  afterEach,
  beforeEach,
  describe,
  expect,
  it,
  mock,
  spyOn,
} from 'bun:test'

import type { DevupUILoaderOptions } from '../loader'
import devupUILoader from '../loader'

const wasm = loadCompiler('wasm')

type CodeExtractResult = DevupCompilerOutput
type LoaderThis = ThisParameterType<typeof devupUILoader>
type LoaderCallback = ReturnType<LoaderThis['async']>
interface TestLoaderContext extends Pick<
  LoaderThis,
  'async' | 'resourcePath' | 'addDependency'
> {
  getOptions: () => Partial<DevupUILoaderOptions>
  _compiler?: { __DEVUP_CACHE: string }
}

function createCodeExtractResult(
  overrides: Partial<CodeExtractResult> = {},
): CodeExtractResult {
  return {
    code: '',
    css: '',
    cssFile: undefined,
    updatedBaseStyle: false,
    map: undefined,
    dependencies: [],
    free: () => {},
    [Symbol.dispose]: () => {},
    ...overrides,
  }
}

function createLoaderContext(
  options: Partial<DevupUILoaderOptions>,
  callback: LoaderCallback,
  resourcePath = 'index.tsx',
  overrides: Partial<TestLoaderContext> = {},
): LoaderThis {
  return {
    getOptions: () => ({ engine: 'wasm', ...options }),
    async: mock().mockReturnValue(callback),
    resourcePath,
    addDependency: mock(),
    ...overrides,
  } as unknown as LoaderThis
}

let codeExtractSpy: ReturnType<typeof spyOn>
let exportClassMapSpy: ReturnType<typeof spyOn>
let exportFileMapSpy: ReturnType<typeof spyOn>
let exportSheetSpy: ReturnType<typeof spyOn>
let getCssSpy: ReturnType<typeof spyOn>
let getDefaultThemeSpy: ReturnType<typeof spyOn>
let getThemeInterfaceSpy: ReturnType<typeof spyOn>
let importClassMapSpy: ReturnType<typeof spyOn>
let importFileMapSpy: ReturnType<typeof spyOn>
let importSheetSpy: ReturnType<typeof spyOn>
let registerThemeSpy: ReturnType<typeof spyOn>
let setDebugSpy: ReturnType<typeof spyOn>
let setPrefixSpy: ReturnType<typeof spyOn>
let writeFileSpy: ReturnType<typeof spyOn>
let dateNowSpy: ReturnType<typeof spyOn>

beforeEach(() => {
  codeExtractSpy = spyOn(wasm, 'codeExtract').mockReturnValue(
    createCodeExtractResult(),
  )
  exportClassMapSpy = spyOn(wasm, 'exportClassMap').mockReturnValue('{}')
  exportFileMapSpy = spyOn(wasm, 'exportFileMap').mockReturnValue('{}')
  exportSheetSpy = spyOn(wasm, 'exportSheet').mockReturnValue('{}')
  getCssSpy = spyOn(wasm, 'getCss').mockReturnValue('')
  getDefaultThemeSpy = spyOn(wasm, 'getDefaultTheme').mockReturnValue(undefined)
  getThemeInterfaceSpy = spyOn(wasm, 'getThemeInterface').mockReturnValue('')
  importClassMapSpy = spyOn(wasm, 'importClassMap').mockReturnValue(undefined)
  importFileMapSpy = spyOn(wasm, 'importFileMap').mockReturnValue(undefined)
  importSheetSpy = spyOn(wasm, 'importSheet').mockReturnValue(undefined)
  registerThemeSpy = spyOn(wasm, 'registerTheme').mockReturnValue(undefined)
  setDebugSpy = spyOn(wasm, 'setDebug').mockReturnValue(undefined)
  setPrefixSpy = spyOn(wasm, 'setPrefix').mockReturnValue(undefined)
  writeFileSpy = spyOn(fsPromises, 'writeFile').mockResolvedValue(undefined)
  dateNowSpy = spyOn(Date, 'now').mockReturnValue(0)
})

afterEach(() => {
  codeExtractSpy.mockRestore()
  exportClassMapSpy.mockRestore()
  exportFileMapSpy.mockRestore()
  exportSheetSpy.mockRestore()
  getCssSpy.mockRestore()
  getDefaultThemeSpy.mockRestore()
  getThemeInterfaceSpy.mockRestore()
  importClassMapSpy.mockRestore()
  importFileMapSpy.mockRestore()
  importSheetSpy.mockRestore()
  registerThemeSpy.mockRestore()
  setDebugSpy.mockRestore()
  setPrefixSpy.mockRestore()
  writeFileSpy.mockRestore()
  dateNowSpy.mockRestore()
})

const waitFor = async (fn: () => void, timeout = 1000) => {
  const start = performance.now()
  while (performance.now() - start < timeout) {
    try {
      fn()
      return
    } catch {
      await new Promise((r) => setTimeout(r, 10))
    }
  }
  fn()
}

describe('devupUILoader', () => {
  it.each(['snapshot', 'map', 'success'] as const)(
    'releases output exactly once when %s completes',
    async (stage) => {
      // Given
      let released = false
      const free = mock(() => {
        released = true
      })
      const failure = new Error('getter failed')
      codeExtractSpy.mockReturnValue({
        ...createCodeExtractResult(),
        get code() {
          expect(released).toBe(false)
          if (stage === 'snapshot') throw failure
          return 'detached code'
        },
        map: stage === 'map' ? '{' : undefined,
        free,
      })
      const callback = mock()
      const context = createLoaderContext(
        { package: 'package', cssDir: 'df', singleCss: true },
        callback,
      )
      // When
      devupUILoader.call(context, Buffer.from('source'), undefined)
      await Promise.resolve()
      // Then
      expect(free).toHaveBeenCalledTimes(1)
      expect(released).toBe(true)
      switch (stage) {
        case 'snapshot':
          expect(callback).toHaveBeenCalledWith(failure)
          break
        case 'map':
          expect(callback).toHaveBeenCalledWith(expect.any(SyntaxError))
          break
        case 'success':
          expect(callback).toHaveBeenCalledWith(null, 'detached code', null)
          break
      }
    },
  )

  it('resolves imports to cwd-relative ids and depends on the modules read', async () => {
    const setModuleResolverSpy = spyOn(
      wasm,
      'setModuleResolver',
    ).mockReturnValue(undefined)
    try {
      codeExtractSpy.mockReturnValue(
        createCodeExtractResult({ dependencies: ['src/tokens.ts'] }),
      )
      const callback = mock()
      const t = createLoaderContext(
        {
          package: 'package',
          cssDir: 'cssDir',
          sheetFile: 'sheetFile',
          classMapFile: 'classMapFile',
          fileMapFile: 'fileMapFile',
          watch: false,
          singleCss: true,
        },
        callback,
      )
      devupUILoader.bind(t)(Buffer.from('code'), 'index.tsx')
      await waitFor(() => expect(callback).toHaveBeenCalled())
      expect(t.addDependency).toHaveBeenCalledWith(
        nodePath.resolve('src/tokens.ts'),
      )
      const resolveModule = setModuleResolverSpy.mock.calls[0][0] as (
        specifier: string,
        importer: string,
      ) => { path: string } | undefined
      expect(resolveModule('./loader.test', import.meta.path)?.path).toBe(
        nodePath
          .relative(process.cwd(), import.meta.path)
          .replaceAll('\\', '/'),
      )
    } finally {
      setModuleResolverSpy.mockRestore()
    }
  })
  it.each(
    createTestMatrix({
      updatedBaseStyle: [true, false],
    }),
  )('should extract code with css', async (options) => {
    const _compiler = {
      __DEVUP_CACHE: '',
    }
    const asyncCallback = mock()
    const t = createLoaderContext(
      {
        package: 'package',
        cssDir: 'cssFile',
        sheetFile: 'sheetFile',
        classMapFile: 'classMapFile',
        fileMapFile: 'fileMapFile',
        watch: true,
        singleCss: true,
      },
      asyncCallback,
      'index.tsx',
      { _compiler },
    )
    exportSheetSpy.mockReturnValue('sheet')
    exportClassMapSpy.mockReturnValue('classMap')
    exportFileMapSpy.mockReturnValue('fileMap')
    getCssSpy.mockReturnValue('css')
    codeExtractSpy.mockReturnValue(
      createCodeExtractResult({
        code: 'code',
        css: 'css',
        map: '{"version":3,"names":[],"sources":["index.tsx"],"mappings":"AAAA"}',
        cssFile: 'cssFile',
        updatedBaseStyle: options.updatedBaseStyle,
      }),
    )
    devupUILoader.bind(t)(Buffer.from('code'), 'index.tsx')

    expect(t.async).toHaveBeenCalled()
    expect(codeExtractSpy).toHaveBeenCalledWith(
      'index.tsx',
      'code',
      'package',
      './cssFile',
      true,
      false,
      true,
      {},
    )
    if (options.updatedBaseStyle) {
      expect(writeFileSpy).toHaveBeenCalledWith(
        join('cssFile', 'devup-ui.css'),
        'css',
        'utf-8',
      )
    } else {
      expect(writeFileSpy).not.toHaveBeenCalledWith(
        join('cssFile', 'devup-ui.css'),
        'css',
        'utf-8',
      )
    }
    await waitFor(() => {
      expect(asyncCallback).toHaveBeenCalledWith(null, 'code', {
        version: 3,
        names: [],
        sources: ['index.tsx'],
        mappings: 'AAAA',
      })
      expect(writeFileSpy).toHaveBeenCalledWith(
        join('cssFile', 'cssFile'),
        '/* cssFile 0 */',
      )
      expect(writeFileSpy).toHaveBeenCalledWith('sheetFile', 'sheet')
      expect(writeFileSpy).toHaveBeenCalledWith('classMapFile', 'classMap')
      expect(writeFileSpy).toHaveBeenCalledWith('fileMapFile', 'fileMap')
    })
  })

  it('should extract code without css', async () => {
    const asyncCallback = mock()
    const t = createLoaderContext(
      {
        package: 'package',
        cssDir: 'cssFile',
        watch: false,
        singleCss: true,
      },
      asyncCallback,
    )
    codeExtractSpy.mockReturnValue(
      createCodeExtractResult({
        code: 'code',
        css: undefined,
        map: undefined,
        cssFile: undefined,
        updatedBaseStyle: false,
      }),
    )
    devupUILoader.bind(t)(Buffer.from('code'), 'index.tsx')

    expect(t.async).toHaveBeenCalled()
    expect(codeExtractSpy).toHaveBeenCalledWith(
      'index.tsx',
      'code',
      'package',
      './cssFile',
      true,
      false,
      true,
      {},
    )
    await waitFor(() => {
      expect(asyncCallback).toHaveBeenCalledWith(null, 'code', null)
    })
    expect(writeFileSpy).not.toHaveBeenCalledWith('cssFile', 'css', {
      encoding: 'utf-8',
    })
  })

  it('extracts a file that reaches Devup UI only through a project module', async () => {
    const source = `import { Box } from './ui'
export const a = <Box bg="red" />`
    const asyncCallback = mock()
    const t = createLoaderContext(
      {
        package: 'package',
        cssDir: 'cssFile',
        watch: false,
        singleCss: true,
      },
      asyncCallback,
    )
    codeExtractSpy.mockReturnValue(
      createCodeExtractResult({
        code: '<div className="a" />',
        css: undefined,
        map: undefined,
        cssFile: undefined,
        updatedBaseStyle: false,
      }),
    )
    devupUILoader.bind(t)(Buffer.from(source), 'index.tsx')

    expect(codeExtractSpy.mock.calls.at(-1)?.[1]).toBe(source)
    await waitFor(() => {
      expect(asyncCallback).toHaveBeenCalledWith(
        null,
        '<div className="a" />',
        null,
      )
    })
  })

  it('extracts a file that only the Emotion css prop compiles (pragma, no Devup UI import)', async () => {
    const source = `/** @jsxImportSource @emotion/react */
export const a = <div css={{ color: 'red' }} />`
    const asyncCallback = mock()
    const t = createLoaderContext(
      {
        package: 'package',
        cssDir: 'cssFile',
        watch: false,
        singleCss: true,
      },
      asyncCallback,
    )
    codeExtractSpy.mockReturnValue(
      createCodeExtractResult({
        code: '<div className="a" />',
        css: undefined,
        map: undefined,
        cssFile: undefined,
        updatedBaseStyle: false,
      }),
    )
    devupUILoader.bind(t)(Buffer.from(source), 'index.tsx')

    expect(codeExtractSpy.mock.calls.at(-1)?.[1]).toBe(source)
    await waitFor(() => {
      expect(asyncCallback).toHaveBeenCalledWith(
        null,
        '<div className="a" />',
        null,
      )
    })
  })

  it('should handle error', async () => {
    const asyncCallback = mock()
    const t = createLoaderContext(
      {
        package: 'package',
        cssDir: 'cssFile',
        watch: false,
        singleCss: true,
      },
      asyncCallback,
    )
    codeExtractSpy.mockImplementation(() => {
      throw new Error('error')
    })
    devupUILoader.bind(t)(Buffer.from('code'), 'index.tsx')

    expect(t.async).toHaveBeenCalled()
    expect(asyncCallback).toHaveBeenCalledWith(new Error('error'))
  })

  it('should propagate css write failures', async () => {
    const asyncCallback = mock()
    const writeError = new Error('write failed')
    const t = createLoaderContext(
      {
        package: 'package',
        cssDir: 'cssFile',
        sheetFile: 'sheetFile',
        classMapFile: 'classMapFile',
        fileMapFile: 'fileMapFile',
        watch: true,
        singleCss: true,
      },
      asyncCallback,
    )
    writeFileSpy.mockRejectedValueOnce(writeError)
    codeExtractSpy.mockReturnValue(
      createCodeExtractResult({
        code: 'code',
        css: 'css',
        map: '{}',
        cssFile: 'cssFile',
        updatedBaseStyle: false,
      }),
    )

    devupUILoader.bind(t)(Buffer.from('code'), 'index.tsx')

    await waitFor(() => {
      expect(asyncCallback).toHaveBeenCalledWith(writeError)
    })
    expect(asyncCallback).not.toHaveBeenCalledWith(null, 'code', {})
  })

  it.each([
    ['src/A.css.ts', 'src/B.css.ts'],
    ['src/B.css.ts', 'src/A.css.ts'],
  ])(
    'keeps the collapsed bucket sentinel identical for %s then %s',
    async (first, second) => {
      codeExtractSpy.mockReturnValue(
        createCodeExtractResult({
          code: 'code',
          css: '.shared{color:red}',
          cssFile: '../devup-ui-23.css',
        }),
      )
      const cssDir = 'sentinel-css'
      for (const resourcePath of [first, second]) {
        const callback = mock()
        const context = createLoaderContext(
          {
            package: 'package',
            cssDir,
            watch: true,
            singleCss: false,
            sheetFile: 'sheet.json',
            classMapFile: 'class-map.json',
            fileMapFile: 'file-map.json',
          },
          callback,
          resourcePath,
        )
        devupUILoader.call(context, Buffer.from('same final source'), undefined)
        await waitFor(() =>
          expect(callback).toHaveBeenCalledWith(null, 'code', null),
        )
      }
      const writes = writeFileSpy.mock.calls.filter(
        ([path]) => path === join(cssDir, 'devup-ui-23.css'),
      )
      expect(writes.at(-1)?.[1]).toBe('/* devup-ui-23.css 0 */')
    },
  )

  it('should load with date now on watch', async () => {
    const asyncCallback = mock()
    const t = createLoaderContext(
      {
        package: 'package',
        cssDir: 'cssFile',
        watch: true,
        singleCss: true,
      },
      asyncCallback,
    )
    codeExtractSpy.mockReturnValue(
      createCodeExtractResult({
        code: 'code',
        css: 'css',
        map: undefined,
        cssFile: 'cssFile',
        updatedBaseStyle: false,
      }),
    )
    devupUILoader.bind(t)(Buffer.from('code'), 'index.tsx')

    expect(t.async).toHaveBeenCalled()
    expect(codeExtractSpy).toHaveBeenCalledWith(
      'index.tsx',
      'code',
      'package',
      './cssFile',
      true,
      false,
      true,
      {},
    )
  })

  it('should load with nowatch', async () => {
    const asyncCallback = mock()
    const t = createLoaderContext(
      {
        package: 'package',
        cssDir: './foo',
        watch: false,
        singleCss: true,
      },
      asyncCallback,
      './foo/index.tsx',
    )
    codeExtractSpy.mockReturnValue(
      createCodeExtractResult({
        code: 'code',
        css: 'css',
        map: undefined,
        cssFile: 'cssFile',
        updatedBaseStyle: false,
      }),
    )
    devupUILoader.bind(t)(Buffer.from('code'), '/foo/index.tsx')
  })
  it('should load with theme', async () => {
    const asyncCallback = mock()
    const t = createLoaderContext(
      {
        package: 'package',
        cssDir: 'cssFile',
        watch: false,
        singleCss: true,
        theme: {
          colors: {
            primary: '#000',
          },
        },
      },
      asyncCallback,
    )
    registerThemeSpy.mockReturnValueOnce(undefined)
    codeExtractSpy.mockReturnValue(
      createCodeExtractResult({
        code: 'code',
        css: 'css',
        map: undefined,
        cssFile: 'cssFile',
        updatedBaseStyle: false,
      }),
    )
    devupUILoader.bind(t)(Buffer.from('code'), 'index.tsx')
  })

  it('posix-normalizes the extraction filename (Windows path safety)', () => {
    // Simulate Windows: force path.relative to emit backslashes. The loader
    // MUST normalize to forward slashes so the engine bucket key matches the
    // canonical map / FILE_ROUTES (built posix by plugin-utils).
    const relativeSpy = spyOn(nodePath, 'relative').mockImplementation(
      (from: string, to: string) =>
        (
          nodePath as { posix: { relative: (a: string, b: string) => string } }
        ).posix
          .relative(from, to)
          .replaceAll('/', '\\'),
    )
    const asyncCallback = mock()
    const t = createLoaderContext(
      {
        package: 'package',
        cssDir: 'df/devup-ui',
        sheetFile: 's',
        classMapFile: 'c',
        fileMapFile: 'f',
        watch: false,
        singleCss: true,
      },
      asyncCallback,
      'src/Card.tsx',
    )
    try {
      devupUILoader.bind(t)(Buffer.from('code'), 'src/Card.tsx')
      const filenameArg = codeExtractSpy.mock.calls[0]?.[0] as string
      expect(filenameArg).not.toContain('\\')
    } finally {
      relativeSpy.mockRestore()
    }
  })
})
