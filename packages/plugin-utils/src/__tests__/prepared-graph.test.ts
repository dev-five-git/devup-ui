import { expect, it } from 'bun:test'

import {
  __setOxcParserForTest,
  buildStaticImportGraph,
  computeCompiledFiles,
  createModuleResolver,
} from '../import-graph'
import { createPreparedFixture } from './prepared-graph-fixture'

let root: string
const file = createPreparedFixture((directory) => {
  root = directory
})

it('reads raw source only for undefined, not for an empty prepared string', async () => {
  const entry = file('src/page.ts', "import './real'")
  const real = file('src/real.ts')
  const raw = await buildStaticImportGraph('src', undefined, {
    cwd: root,
    prepareSource: () => undefined,
  })
  const empty = await buildStaticImportGraph('src', undefined, {
    cwd: root,
    prepareSource: () => '',
  })
  expect(raw.staticImports.get(entry)).toEqual(new Set([real]))
  expect(empty.staticImports.get(entry)).toEqual(new Set())
})

it.each([false, true])(
  'wraps located preparer throws/rejections (async %s)',
  async (asyncHook) => {
    const entry = file('src/main.ts')
    const cause = Object.assign(new Error('Compiler failed'), {
      line: 3,
      column: 4,
    })
    const hook = () => {
      throw cause
    }
    const promise = buildStaticImportGraph('src', undefined, {
      cwd: root,
      prepareSource: asyncHook ? async () => hook() : hook,
    })
    expect(promise).toBeInstanceOf(Promise)
    const error = await promise.catch((failure: unknown) => failure)
    if (!(error instanceof Error)) throw new Error('Expected error')
    expect(error.message).toBe(
      `${entry}:3:4 (in compiled output): Graph source preparation failed: Compiler failed`,
    )
    expect(error.cause).toBe(cause)
  },
)

it('turns asynchronous setup faults into Promise rejections', async () => {
  const config = file('tsconfig.json', '{')
  const promise = buildStaticImportGraph('src', config, {
    cwd: root,
    prepareSource: () => '',
  })
  expect(promise).toBeInstanceOf(Promise)
  await expect(promise).rejects.toThrow(config)
})

it.each([
  undefined,
  { version: 3, sources: ['original.mdx'], names: [], mappings: 'AAAA;AACA' },
])('reports every available parser diagnostic with map %j', async (map) => {
  const entry = file('src/page.mdx')
  __setOxcParserForTest({
    parseSync: () => ({
      errors: [
        { message: 'first', line: 1, column: 1 },
        { message: 'second', line: 2, column: 1 },
      ],
    }),
  })
  const promise = buildStaticImportGraph('src', undefined, {
    cwd: root,
    includeMdx: true,
    prepareSource: () => ({ code: 'compiled', map }),
  })
  const error = await promise.catch((cause: unknown) => cause)
  if (!(error instanceof Error)) throw new Error('Expected diagnostics')
  expect(error.message).toContain(
    map ? 'original.mdx:1:1' : `${entry}:1:1 (in compiled output)`,
  )
  expect(error.message).toContain(
    map ? 'original.mdx:2:1' : `${entry}:2:1 (in compiled output)`,
  )
  if (map) expect(error.message).not.toContain('(in compiled output)')
})

it('does not fall back to raw reads or lexical scanning after a prepared parser throws', async () => {
  const entry = file('src/page.mdx', "import './raw'")
  const cause = new Error(`${entry}:2:3: parse failed`)
  __setOxcParserForTest({
    parseSync: () => {
      throw cause
    },
  })
  await expect(
    buildStaticImportGraph('src', undefined, {
      cwd: root,
      includeMdx: true,
      prepareSource: () => "import './compiled'",
    }),
  ).rejects.toThrow(`${entry}:2:3 (in compiled output)`)
})

it('uses the available AST result for prepared source', async () => {
  const entry = file('src/page.mdx')
  const leaf = file('src/leaf.ts')
  __setOxcParserForTest({
    parseSync: () => ({
      type: 'ImportDeclaration',
      source: { value: './leaf' },
    }),
  })
  const graph = await buildStaticImportGraph('src', undefined, {
    cwd: root,
    includeMdx: true,
    prepareSource: () => 'compiled',
  })
  expect(graph.staticImports.get(entry)).toEqual(new Set([leaf]))
})

it('reads the documented available-parser program getter for prepared source', async () => {
  const entry = file('src/page.mdx')
  const leaf = file('src/leaf.ts')
  const result = Object.create({
    get program() {
      return { type: 'ImportDeclaration', source: { value: './leaf' } }
    },
    get errors() {
      return []
    },
  })
  __setOxcParserForTest({ parseSync: () => result })
  const graph = await buildStaticImportGraph('src', undefined, {
    cwd: root,
    includeMdx: true,
    prepareSource: () => '',
  })
  expect(graph.staticImports.get(entry)).toEqual(new Set([leaf]))
})

it('reports every documented available-parser diagnostic label and remaps each position', async () => {
  const entry = file('src/page.mdx')
  __setOxcParserForTest({
    parseSync: () => ({
      errors: [
        {
          message: 'labels',
          labels: [
            { start: 0, end: 1, message: 'first' },
            { start: 2, end: 3, message: null },
          ],
        },
      ],
    }),
  })
  const map = {
    version: 3,
    sources: ['original.mdx'],
    names: [],
    mappings: 'AAAA;AACA',
  }
  const error = await buildStaticImportGraph('src', undefined, {
    cwd: root,
    includeMdx: true,
    prepareSource: () => ({ code: 'a\nb', map }),
  }).catch((cause: unknown) => cause)
  if (!(error instanceof Error)) throw new Error('Expected diagnostics')
  expect(error.message).toContain('original.mdx:1:1')
  expect(error.message).toContain('original.mdx:2:1')
  expect(error.message).not.toContain(`${entry}:`)
  expect(error.message).not.toContain('(in compiled output)')
})

it('selects Markdown routes and extensionless imports only with exact opt-in', () => {
  const entry = file('src/page.md', "import './leaf'\n\n# Heading")
  const leaf = file('src/leaf.md', '# Leaf')
  file('src/other.mdx')
  expect(() => createModuleResolver({ cwd: root })('./leaf.md', entry)).toThrow(
    leaf,
  )
  expect(createModuleResolver({ cwd: root })('./leaf', entry)).toBeUndefined()
  const graph = buildStaticImportGraph('src', undefined, {
    cwd: root,
    includeMdx: ['.md'],
  })
  expect(graph.staticImports.get(entry)).toEqual(new Set([leaf]))
  expect(computeCompiledFiles({ cwd: root, srcDir: 'src', graph })).toEqual([
    'src/leaf.md',
    'src/page.md',
  ])
})

it('rejects unprepared legacy extensionless MDX while the graph remains default-off', () => {
  const entry = file('src/main.ts', "import './page'")
  const page = file('src/page.mdx', '# Heading')
  expect(() => createModuleResolver({ cwd: root })('./page', entry)).toThrow(
    page,
  )
  expect(buildStaticImportGraph('src', undefined, { cwd: root }).files).toEqual(
    [entry],
  )
})
