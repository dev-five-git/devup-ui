import { expect, it } from 'bun:test'

import { scanImports } from '../import-scanner'
import { buildStaticImportGraph, importGraphFailureOf } from '../index'
import { createPreparedFixture } from './prepared-graph-fixture'

let root: string
const file = createPreparedFixture((directory) => {
  root = directory
})

it('preserves raw Markdown offsets through prose, fences, comments and CRLF', () => {
  // Given an ESM block after masked Markdown.
  const source =
    '# Heading\r\n\r\n```js\r\nimport "fake"\r\n```\r\n<!-- hidden -->\r\n\r\nimport "./leaf"\r\n\r\nProse'
  const entry = file('src/page.mdx', source)
  const leaf = file('src/leaf.ts')
  // When scanning raw opted-in Markdown.
  const graph = buildStaticImportGraph('src', undefined, {
    cwd: root,
    includeMdx: true,
  })
  // Then coordinates refer to the actual source, not compacted ESM blocks.
  expect(graph.requests).toEqual([
    {
      importer: entry,
      request: './leaf',
      specifier: './leaf',
      kind: 'static-import',
      position: { offset: 66, line: 8, column: 8 },
      source: 'source',
      outcome: { kind: 'resolved', path: leaf },
    },
  ])
})

it.each([
  undefined,
  { version: 3, sources: ['original.mdx'], names: [], mappings: 'AAAA' },
])('labels prepared token positions honestly with map %j', async (map) => {
  // Given different raw and compiled text under the real Markdown filename.
  const entry = file('src/page.mdx', '# Raw')
  const leaf = file('src/leaf.ts')
  const visits: string[] = []
  // When the unchanged asynchronous driver prepares actual JavaScript.
  const graph = await buildStaticImportGraph('src', undefined, {
    cwd: root,
    includeMdx: true,
    prepareSource: (filename) => {
      visits.push(filename)
      return filename === entry
        ? { code: '\nimport "./leaf"', sourceType: 'compiled-mdx', map }
        : undefined
    },
  })
  // Then compiled coordinates and caller-owned map are not claimed as raw positions.
  expect(graph.requests).toEqual([
    {
      importer: entry,
      request: './leaf',
      specifier: './leaf',
      kind: 'static-import',
      position: { offset: 8, line: 2, column: 8 },
      source: 'compiled',
      map,
      outcome: { kind: 'resolved', path: leaf },
    },
  ])
  expect(visits.sort()).toEqual(graph.files)
})

it.each(['.js', '.tsx', '.ts'])(
  'preserves scanner masks and type-only/nonliteral parity in %s',
  (extension) => {
    // Given text, expressions, type syntax and nonliteral calls.
    const source =
      extension === '.js'
        ? 'const x = a < b; import("external"); require(name)'
        : extension === '.tsx'
          ? 'const x = <div title="fake">import("fake"){require("external")}</div>; import type { T } from "types"'
          : 'type T = import("types").T; const x = <number>a; import("external"); import(name)'
    const entry = file(`src/main${extension}`, source)
    // When the graph uses the same lexer context as the legacy scanner.
    const graph = buildStaticImportGraph('src', undefined, { cwd: root })
    // Then only the runtime literal occurrence survives and old object shape is exact.
    expect(graph.requests.map(({ specifier }) => specifier)).toEqual([
      'external',
    ])
    expect(
      scanImports(source, extension === '.tsx', extension !== '.js'),
    ).toEqual([
      {
        kind: extension === '.tsx' ? 'static' : 'dynamic',
        specifier: 'external',
      },
    ])
    expect(graph.requests[0]?.importer).toBe(entry)
  },
)

it('keeps explicit compiled-MDX JavaScript grammar under a TS filename', async () => {
  // Given JavaScript using the value identifier type with JSX.
  const entry = file('src/main.ts')
  // When preparation explicitly selects JS/JSX.
  const graph = await buildStaticImportGraph('src', undefined, {
    cwd: root,
    prepareSource: () => ({
      code: 'const type = 1; const x = <div>{import("external")}</div>',
      sourceType: 'compiled-mdx',
    }),
  })
  // Then the actual compiled literal is retained under its real filename.
  expect(
    graph.requests.map(({ importer, specifier, source }) => ({
      importer,
      specifier,
      source,
    })),
  ).toEqual([{ importer: entry, specifier: 'external', source: 'compiled' }])
})

it('does not invent failed-edge evidence for preparation errors before any request', async () => {
  // Given a hook failure before scanning.
  file('src/page.mdx')
  // When the established driver wraps the compiler failure.
  const failure = await buildStaticImportGraph('src', undefined, {
    cwd: root,
    includeMdx: true,
    prepareSource: () => {
      throw new Error('compiler')
    },
  }).catch((error: unknown) => error)
  // Then no synthetic scanner edge is attributed to it.
  expect(importGraphFailureOf(failure)).toBeUndefined()
})

it.each([
  ['\r\n', 19],
  ['\r', 18],
  ['\n', 18],
  ['\u2028', 18],
  ['\u2029', 18],
] as const)(
  'locates raw requests after line terminator %j',
  (separator, offset) => {
    // Given actual source line terminators, not literal backslash escapes.
    const source = `const n=0;${separator}import 'external';`
    const entry = file('src/main.ts', source)
    // When scanning the raw source through the public graph.
    const graph = buildStaticImportGraph('src', undefined, { cwd: root })
    // Then independently known coordinates and legacy selection are preserved.
    expect(graph.requests).toEqual([
      {
        importer: entry,
        request: 'external',
        specifier: 'external',
        kind: 'static-import',
        position: { offset, line: 2, column: 8 },
        source: 'source',
        outcome: { kind: 'external', request: 'external' },
      },
    ])
    expect(scanImports(source, false)).toEqual([
      { kind: 'static', specifier: 'external' },
    ])
  },
)

it.each(['\u2028', '\u2029'])(
  'locates prepared requests after Unicode line terminator %j',
  async (separator) => {
    // Given compiled text with an actual Unicode line terminator.
    const entry = file('src/main.ts', 'const raw = 1;')
    // When preparation supplies the exact audit witness.
    const graph = await buildStaticImportGraph('src', undefined, {
      cwd: root,
      prepareSource: () => `const n=0;${separator}import 'external';`,
    })
    // Then the position describes compiled text, not the raw file.
    expect(graph.requests[0]).toMatchObject({
      importer: entry,
      request: 'external',
      position: { offset: 18, line: 2, column: 8 },
      source: 'compiled',
      outcome: { kind: 'external', request: 'external' },
    })
  },
)

it.each([
  ['\u2028', false],
  ['\u2029', false],
  ['\u2028', true],
  ['\u2029', true],
] as const)(
  'locates failing requests after Unicode line terminator %j with preparation %j',
  async (separator, prepared) => {
    // Given a terminal alias miss in raw or compiled witness text.
    const source = `const n=0;${separator}import 'external';`
    const entry = file('src/main.ts', prepared ? 'const raw = 1;' : source)
    // When the existing resolver rejects the requested alias.
    let caught: unknown
    try {
      await buildStaticImportGraph('src', undefined, {
        cwd: root,
        alias: { external: './absent' },
        ...(prepared ? { prepareSource: () => source } : {}),
      })
    } catch (error) {
      caught = error
    }
    // Then failing-edge evidence retains the independently known position.
    expect(caught).toBeInstanceOf(Error)
    expect(importGraphFailureOf(caught)).toEqual([
      {
        importer: entry,
        request: 'external',
        specifier: 'external',
        kind: 'static-import',
        position: { offset: 18, line: 2, column: 8 },
        source: prepared ? 'compiled' : 'source',
        ...(prepared ? { map: undefined } : {}),
        outcome: { kind: 'error', error: caught },
      },
    ])
  },
)
