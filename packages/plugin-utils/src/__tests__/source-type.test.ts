import { createRequire } from 'node:module'
import { relative, resolve } from 'node:path'

import { afterEach, beforeEach, expect, it } from 'bun:test'

import * as wasm from '../../../../bindings/devup-ui-wasm/pkg'
import {
  __setOxcParserForTest,
  buildStaticImportGraph,
  createModuleResolver,
  type PreparedSource,
} from '../import-graph'
import { PreparedSourceTypeError, readPreparedSource } from '../prepared-source'
import { createPreparedFixture } from './prepared-graph-fixture'

const projectRequire = createRequire(resolve('apps/landing/package.json'))
const compilerRequire = createRequire(projectRequire.resolve('@mdx-js/loader'))
const { compile } = compilerRequire('@mdx-js/mdx')
let root: string
const file = createPreparedFixture((directory) => {
  root = directory
})
let debug: boolean
beforeEach(() => {
  debug = wasm.isDebug()
  wasm.resetBuildState()
  wasm.setDebug(false)
})
afterEach(() => {
  wasm.resetBuildState()
  wasm.setDebug(debug)
  __setOxcParserForTest(undefined)
})

it.each([false, true])(
  'extracts compiled custom-extension exports after JSX (%s)',
  async (jsx) => {
    const markdown =
      'import {Box} from "@devup-ui/react"\n\n<Box bg="tomato" />'
    const module = file('src/value.mdown', markdown)
    const entry = file('src/main.tsx', 'import {PRIMARY} from "./value.mdown";')
    const compiled = await compile(
      { path: module, value: markdown },
      { format: 'mdx', jsx },
    )
    const source: PreparedSource = {
      code: `${compiled}\nexport const PRIMARY = 'blue';`,
      sourceType: 'compiled-mdx',
      map: compiled.map,
    }
    const prepareSource = (filename: string) =>
      filename === module ? source : undefined
    const toId = (filename: string) =>
      relative(root, filename).replaceAll('\\', '/')
    const resolver = createModuleResolver({
      cwd: root,
      includeMdx: ['.mdown'],
      prepareSource,
      toId,
    })
    const graph = await buildStaticImportGraph('src', undefined, {
      cwd: root,
      includeMdx: ['.mdown'],
      prepareSource,
    })
    expect(graph.staticImports.get(entry)).toEqual(new Set([module]))
    expect(resolver('./value.mdown', toId(entry))?.sourceType).toBe(
      'compiled-mdx',
    )
    wasm.setModuleResolver(resolver)
    const output = wasm.codeExtractWithoutSourceMap(
      toId(entry),
      'import {Box} from "@devup-ui/react"; import {PRIMARY} from "./value.mdown"; export const view = <Box color={PRIMARY}/>;',
      '@devup-ui/react',
      'df',
      true,
      false,
      false,
      {},
    )
    try {
      expect(wasm.getCss(undefined, false)).toContain('color:blue')
      expect(output.code).not.toContain('--')
      expect(output.dependencies).toEqual([toId(module)])
    } finally {
      output.free()
    }
  },
)

it('uses JS JSX scanner and diagnostics when explicit mode overrides a TS filename', async () => {
  const entry = file('src/page.ts')
  const real = file('src/real.js')
  file('node_modules/phantom/package.json', '{')
  const modes: unknown[] = []
  __setOxcParserForTest({
    parseSync: (_filename, _code, options) => {
      modes.push(options?.lang)
      return {}
    },
  })
  const graph = await buildStaticImportGraph('src', undefined, {
    cwd: root,
    prepareSource: (filename) =>
      filename === entry
        ? {
            code: 'const view = <div>import "phantom"</div>; import "./real.js";',
            sourceType: 'compiled-mdx',
          }
        : undefined,
  })
  expect(graph.staticImports.get(entry)).toEqual(new Set([real]))
  expect(modes).toContain('jsx')
})

it.each(['invalid', 7, null, true, {}])(
  'rejects invalid hook mode before scanner fallback (%p)',
  async (value) => {
    const entry = file('src/page.ts', '')
    const prepared = { code: '' }
    Reflect.set(prepared, 'sourceType', value)
    await expect(
      buildStaticImportGraph('src', undefined, {
        cwd: root,
        prepareSource: () => prepared,
      }),
    ).rejects.toThrow(`${entry}:1:1:`)
    const resolver = createModuleResolver({
      cwd: root,
      prepareSource: () => prepared,
    })
    expect(() => resolver(entry, 'main.ts')).toThrow('main.ts:1:1:')
  },
)

it('preserves getter cause when preparation cannot read optional mode', async () => {
  const entry = file('src/page.ts', '')
  const cause = new Error('sourceType getter fault')
  const prepared = {
    code: '',
    get sourceType(): 'compiled-mdx' {
      throw cause
    },
  }
  await expect(
    buildStaticImportGraph('src', undefined, {
      cwd: root,
      prepareSource: () => prepared,
    }),
  ).rejects.toThrow('sourceType getter fault')
  const resolver = createModuleResolver({
    cwd: root,
    prepareSource: () => prepared,
  })
  expect(() => resolver(entry, 'main.ts')).toThrow('main.ts:1:1:')
})

it('replaces mode when prepared generations become ordinary source', () => {
  const module = file('src/value.ts', 'export const PRIMARY = "red";')
  let source: PreparedSource = {
    code: 'export const PRIMARY = "blue";',
    sourceType: 'compiled-mdx',
  }
  const resolver = createModuleResolver({
    cwd: root,
    prepareSource: () => source,
  })
  expect(resolver(module, 'main.ts')?.sourceType).toBe('compiled-mdx')
  source = { code: '' }
  expect(resolver(module, 'main.ts')).toEqual({ path: module, code: '' })
  source = undefined
  expect(resolver(module, 'main.ts')).toEqual({
    path: module,
    code: 'export const PRIMARY = "red";',
  })
})

it('retains empty/string/undefined preparation contracts when mode is omitted', () => {
  expect(readPreparedSource(undefined)).toBeUndefined()
  expect(readPreparedSource('')).toBe('')
  expect(readPreparedSource({ code: '' })).toEqual({ code: '', map: undefined })
  expect(new PreparedSourceTypeError('getter fault').message).toContain(
    'getter fault',
  )
})
