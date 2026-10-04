import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import * as fsPromises from 'node:fs/promises'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import * as nodePath from 'node:path'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

import { createModuleResolver } from '@devup-ui/plugin-utils'
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

import type { DevupUILoaderOptions } from '../loader'
import devupUILoader from '../loader'

type CodeExtractResult = ReturnType<typeof wasm.codeExtract>
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
    free: () => {},
    [Symbol.dispose]: () => {},
    ...overrides,
  } as unknown as CodeExtractResult
}

function createLoaderContext(
  options: Partial<DevupUILoaderOptions>,
  callback: LoaderCallback,
  resourcePath = 'index.tsx',
  overrides: Partial<TestLoaderContext> = {},
): LoaderThis {
  return {
    getOptions: () => options,
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
  it('keys resolver reuse by the explicit project root and active conditions', async () => {
    const root = mkdtempSync(
      join(process.env.DEVUP_PLUGIN_TEST_TMP ?? tmpdir(), 'loader-'),
    )
    for (const project of ['first', 'second']) {
      const library = join(root, project, 'node_modules', 'conditional')
      mkdirSync(library, { recursive: true })
      writeFileSync(
        join(library, 'package.json'),
        JSON.stringify({
          exports: { browser: './browser.js', node: './node.js' },
        }),
      )
      writeFileSync(join(library, 'browser.js'), project + '-browser')
      writeFileSync(join(library, 'node.js'), project + '-node')
    }
    const selected: (string | undefined)[] = []
    const register = spyOn(wasm, 'setModuleResolver').mockImplementation(
      (resolver: ReturnType<typeof createModuleResolver>) => {
        selected.push(resolver('conditional', 'src/main.ts')?.code)
      },
    )
    try {
      for (const [project, condition] of [
        ['first', 'browser'],
        ['first', 'node'],
        ['second', 'node'],
      ]) {
        const rootDir = join(root, project)
        await new Promise<void>((done) => {
          const context = createLoaderContext(
            {
              rootDir,
              conditions: [condition],
              cssDir: join(rootDir, 'df/devup-ui'),
            },
            () => done(),
            join(rootDir, 'src/main.ts'),
          )
          devupUILoader.bind(context)(Buffer.from('code'))
        })
      }
      expect(selected).toEqual(['first-browser', 'first-node', 'second-node'])
    } finally {
      register.mockRestore()
      rmSync(root, { recursive: true, force: true })
    }
  })
  it('extracts actual compiled MDX while retaining the original resource filename', async () => {
    const landing = createRequire(
      nodePath.resolve(
        import.meta.dir,
        '../../../../apps/landing/package.json',
      ),
    )
    const mdx = await import(
      pathToFileURL(
        createRequire(landing.resolve('@mdx-js/loader')).resolve('@mdx-js/mdx'),
      ).href
    )
    const rootDir = process.cwd()
    const filename = nodePath.resolve(rootDir, 'src/page.mdx')
    const compiled = await mdx.compile({
      value:
        'import {Box} from \'@devup-ui/react\'\n\n# Heading\n\n<Box bg="red" />',
      path: filename,
    })
    codeExtractSpy.mockRestore()
    wasm.resetBuildState()
    const callback = mock()
    const context = createLoaderContext(
      {
        package: '@devup-ui/react',
        cssDir: nodePath.resolve('df/devup-ui'),
        rootDir,
        singleCss: true,
      },
      callback,
      filename,
    )

    await new Promise<void>((done) => {
      callback.mockImplementation(() => done())
      devupUILoader.bind(context)(Buffer.from(String(compiled)))
    })

    expect(callback.mock.calls[0][0]).toBeNull()
    expect(callback.mock.calls[0][1]).not.toContain('bg: "red"')
    expect(callback.mock.calls[0][1]).toContain('devup-ui.css')
  })

  it.each([
    undefined,
    { version: 3, sources: ['authored.mdx'], names: [], mappings: 'AAGE' },
  ])(
    'locates MDX errors with the supplied compiler map %j',
    async (inputMap) => {
      const callback = mock()
      const rootDir = nodePath.resolve('/project')
      const filename = nodePath.join(rootDir, 'page.mdx')
      codeExtractSpy.mockImplementation(() => {
        throw new Error('page.mdx:1:1: cannot extract')
      })
      const context = createLoaderContext(
        { rootDir, cssDir: nodePath.join(rootDir, 'df/devup-ui') },
        callback,
        filename,
      )

      devupUILoader.bind(context)(Buffer.from('compiled'), inputMap)

      expect(callback.mock.calls[0][0].message).toContain(
        inputMap ? 'authored.mdx:4:3' : 'page.mdx:1:1 (in compiled MDX)',
      )
    },
  )
  it('resolves imports to cwd-relative ids and depends on the modules read', async () => {
    const setModuleResolverSpy = spyOn(
      wasm,
      'setModuleResolver',
    ).mockReturnValue(undefined)
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
      nodePath.relative(process.cwd(), import.meta.path).replaceAll('\\', '/'),
    )
    setModuleResolverSpy.mockRestore()
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
        map: '{}',
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
      expect(asyncCallback).toHaveBeenCalledWith(null, 'code', '{}')
      expect(writeFileSpy).toHaveBeenCalledWith(
        join('cssFile', 'cssFile'),
        '/* index.tsx 0 */',
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
