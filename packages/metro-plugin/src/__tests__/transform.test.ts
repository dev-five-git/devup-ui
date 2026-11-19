import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'

import {
  afterEach,
  beforeEach,
  describe,
  expect,
  it,
  mock,
  spyOn,
} from 'bun:test'

import { DevupUIBuildError } from '../located-error'
import type { MetroTransformerSettings } from '../settings'
import {
  type BabelTransformer,
  type BabelTransformerArgs,
  createTransformer,
  engineVersion,
  type NativeCompiler,
  type TransformerHost,
} from '../transform'

let root: string

beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), 'devup-metro-transform-'))
})

afterEach(() => {
  rmSync(root, { recursive: true, force: true })
})

function write(path: string, content: string): string {
  const file = join(root, path)
  mkdirSync(dirname(file), { recursive: true })
  writeFileSync(file, content)
  return file
}

const APP =
  "import { Box } from '@devup-ui/react'\nexport const A = () => <Box p={1} />"

/** A compiler double whose `codeExtractNative` returns `code` */
function fakeCompiler(
  codeExtractNative: (...args: unknown[]) => string = () => 'compiled',
  map: string | null = null,
) {
  const free = mock()
  const extract = mock((...args: unknown[]) => ({
    code: codeExtractNative(...args),
    map,
    free,
  }))
  const compiler: NativeCompiler = { codeExtractNative: extract }
  return { compiler, extract, free }
}

function setup(
  overrides: Partial<MetroTransformerSettings> = {},
  upstream: BabelTransformer = {
    transform: (args) => args,
    getCacheKey: () => 'upstream-key',
  },
  compiler = fakeCompiler().compiler,
) {
  const settings: MetroTransformerSettings = {
    upstreamTransformer: '/upstream.js',
    package: '@devup-ui/react',
    include: [],
    runtime: true,
    devupFile: join(root, 'devup.json'),
    ...overrides,
  }
  const host = {
    settings: mock(() => settings),
    requireModule: mock(() => upstream),
    loadCompiler: mock(() => compiler),
    engineVersion: mock(() => '@devup-ui/binding@1.0.0'),
  } satisfies TransformerHost
  return { host, transformer: createTransformer(host), settings }
}

function args(
  filename: string,
  src = APP,
  platform: string | null = 'ios',
): BabelTransformerArgs {
  return { filename, src, options: { platform, projectRoot: root } }
}

describe('transform', () => {
  it('compiles a file that imports Devup UI and hands it upstream', () => {
    const { compiler, extract, free } = fakeCompiler()
    const transform = mock((input: BabelTransformerArgs) => ({ input }))
    const { host, transformer } = setup(
      { runtime: false, engine: 'wasm' },
      { transform },
      compiler,
    )
    const input = { ...args('src/App.tsx'), plugins: ['p'] }
    expect(transformer.transform(input)).toEqual({
      input: { ...input, src: 'compiled' },
    })
    expect(extract.mock.calls).toEqual([
      ['src/App.tsx', APP, '@devup-ui/react', 'ios', false],
    ])
    expect(free).toHaveBeenCalledTimes(1)
    expect(host.requireModule.mock.calls).toEqual([['/upstream.js']])
    expect(host.loadCompiler.mock.calls).toEqual([['wasm']])
  })

  it('loads the settings, the upstream and the engine once', () => {
    const { host, transformer } = setup()
    transformer.transform(args('a.tsx'))
    transformer.transform(args('b.tsx', APP, 'android'))
    expect(host.settings).toHaveBeenCalledTimes(1)
    expect(host.requireModule).toHaveBeenCalledTimes(1)
    expect(host.loadCompiler).toHaveBeenCalledTimes(1)
  })

  it('passes other files upstream untouched', () => {
    const { host, transformer } = setup({ include: ['lib'] })
    for (const input of [
      args('App.tsx', 'export const a = 1'),
      args('data.json', APP),
      args('node_modules/other/index.js', APP),
      args('node_modules/lib/node_modules/nested/index.js', APP),
    ]) {
      expect(transformer.transform(input)).toBe(input)
    }
    expect(host.loadCompiler).not.toHaveBeenCalled()
  })

  it('compiles included packages, Devup UI ones and a custom package', () => {
    const { compiler, extract } = fakeCompiler()
    const { transformer } = setup(
      { include: ['lib'], package: '@acme/ui' },
      { transform: (input) => input },
      compiler,
    )
    const code = "import { Box } from '@acme/ui'"
    transformer.transform(args('node_modules/lib/index.js', code))
    transformer.transform(args('node_modules/@devup-ui/kit/index.mjs', code))
    transformer.transform(args('App.tsx', APP))
    expect(extract.mock.calls.map(([filename]) => filename)).toEqual([
      'node_modules/lib/index.js',
      'node_modules/@devup-ui/kit/index.mjs',
    ])
  })

  it('moves the locations of the upstream AST to the source', () => {
    // Compiled line 2 is inserted; compiled line 3 is source line 2
    const map = '{"mappings":"AAAA;;AACA"}'
    const { compiler } = fakeCompiler(() => 'compiled', map)
    const loc = { start: { line: 3, column: 1 }, end: { line: 2, column: 0 } }
    const results: unknown[] = [
      { ast: { type: 'File', loc } },
      null,
      'code',
      {},
    ]
    const { transformer } = setup(
      {},
      { transform: () => results.shift() },
      compiler,
    )
    expect(transformer.transform(args('App.tsx'))).toEqual({
      ast: {
        type: 'File',
        loc: { start: { line: 2, column: 1 }, end: { line: 1, column: 0 } },
      },
    })
    expect(transformer.transform(args('App.tsx'))).toBe(null)
    expect(transformer.transform(args('App.tsx'))).toBe('code')
    expect(transformer.transform(args('App.tsx'))).toEqual({})
  })

  it('keeps the arguments when the compiled code is the same', () => {
    const { compiler } = fakeCompiler(() => APP)
    const { transformer } = setup({}, { transform: (input) => input }, compiler)
    const input = args('App.tsx')
    expect(transformer.transform(input)).toBe(input)
  })

  it('passes an empty platform when Metro gives none', () => {
    const { compiler, extract } = fakeCompiler()
    const { transformer } = setup({}, { transform: (input) => input }, compiler)
    transformer.transform(args('App.tsx', APP, null))
    expect(extract.mock.calls[0][3]).toBe('')
  })

  it('reports a build error at its location', () => {
    const { compiler } = fakeCompiler(() => {
      throw new Error('App.tsx:2:24: `<Box>` cannot use `_hover`')
    })
    const { transformer } = setup({}, { transform: (input) => input }, compiler)
    let thrown: unknown
    try {
      transformer.transform(args('App.tsx'))
    } catch (error) {
      thrown = error
    }
    expect(thrown).toBeInstanceOf(DevupUIBuildError)
    expect(thrown).toMatchObject({ loc: { line: 2, column: 23 } })
  })
})

describe('getCacheKey', () => {
  it('changes with the upstream key, devup.json, its extends, the engine and the settings', () => {
    let upstreamKey = 'a'
    const upstream: BabelTransformer = {
      transform: (input) => input,
      getCacheKey: mock(() => upstreamKey),
    }
    const { host, transformer } = setup({}, upstream)
    const keys = new Set<string>()
    const key = () => {
      const value = transformer.getCacheKey({ projectRoot: root })
      keys.add(value)
      return value
    }
    const first = key()
    expect(first).toMatch(/^[0-9a-f]{64}$/)
    expect(key()).toBe(first)
    expect(upstream.getCacheKey).toHaveBeenCalledWith({ projectRoot: root })
    upstreamKey = 'b'
    key()
    const warn = spyOn(console, 'warn').mockImplementation(() => {})
    write('devup.json', '{"extends":["base.json"]}')
    key()
    expect(warn).toHaveBeenCalledTimes(1)
    warn.mockRestore()
    write('base.json', '{"theme":{}}')
    key()
    write('base.json', '{"theme":{"colors":{}}}')
    key()
    write('devup.json', '{')
    key()
    host.engineVersion.mockReturnValue('@devup-ui/binding@2.0.0')
    key()
    expect(keys.size).toBe(7)
  })

  it('works with an upstream transformer without a cache key', () => {
    const { transformer: withKey } = setup(
      {},
      {
        transform: (input) => input,
        getCacheKey: () => '',
      },
    )
    const { transformer } = setup({}, { transform: (input) => input })
    expect(transformer.getCacheKey()).toBe(withKey.getCacheKey())
  })

  it('differs between settings', () => {
    const { transformer: strict } = setup({ runtime: false })
    const { transformer } = setup()
    expect(strict.getCacheKey()).not.toBe(transformer.getCacheKey())
  })
})

describe('engineVersion', () => {
  it('reads the version of the package the engine resolves to', () => {
    write(
      'node_modules/@devup-ui/binding/package.json',
      JSON.stringify({ name: '@devup-ui/binding', version: '1.2.3' }),
    )
    // A nested manifest of another name, as wasm-pack writes into `pkg`
    write(
      'node_modules/@devup-ui/binding/pkg/package.json',
      JSON.stringify({ name: 'devup-ui-binding', version: '0.0.1' }),
    )
    const entry = write('node_modules/@devup-ui/binding/pkg/index.cjs', '')
    const resolve = mock(() => entry)
    expect(engineVersion(undefined, { resolve })).toBe(
      '@devup-ui/binding@1.2.3',
    )
    expect(resolve.mock.calls).toEqual([['@devup-ui/binding']])
  })

  it('names the WASM package for the WASM engine', () => {
    const resolve = mock(() => {
      throw new Error('not found')
    })
    expect(engineVersion('wasm', { resolve })).toBe('@devup-ui/wasm@unknown')
  })

  it('is unknown when no manifest names the package', () => {
    const entry = write('lonely/index.js', '')
    const resolve = mock(() => entry)
    expect(engineVersion('native', { resolve })).toBe(
      '@devup-ui/binding@unknown',
    )
  })
})
