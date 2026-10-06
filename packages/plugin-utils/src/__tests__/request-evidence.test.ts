import * as fs from 'node:fs'
import { dirname, join } from 'node:path'

import { expect, it, spyOn } from 'bun:test'

import { buildStaticImportGraph, importGraphFailureOf } from '../index'
import { createPreparedFixture } from './prepared-graph-fixture'

let root: string
const file = createPreparedFixture((directory) => {
  root = directory
})

it('retains four request kinds, written escapes and duplicate token positions', () => {
  // Given distinct literal occurrences resolving to the same module.
  const source = String.raw`import './\x61';
export * from './a';
import('./a'); require('./a'); import './a';`
  const entry = file('src/page.ts', source)
  const leaf = file('src/a.ts')
  // When building the public graph.
  const graph = buildStaticImportGraph('src', undefined, { cwd: root })
  // Then every occurrence survives with genuine quote-token coordinates.
  expect(graph.requests).toEqual(
    [
      ['static-import', './\\x61', 7, 1, 8],
      ['re-export', './a', 31, 2, 15],
      ['literal-dynamic-import', './a', 45, 3, 8],
      ['literal-require', './a', 61, 3, 24],
      ['static-import', './a', 76, 3, 39],
    ].map(([kind, request, offset, line, column]) => ({
      importer: entry,
      kind,
      request,
      specifier: './a',
      position: { offset, line, column },
      source: 'source',
      outcome: { kind: 'resolved', path: leaf },
    })),
  )
  expect(graph.staticImports.get(entry)).toEqual(new Set([leaf]))
  expect(graph.dynamicImports.get(entry)).toEqual(new Set([leaf]))
  expect(Object.isFrozen(graph.requests)).toBe(true)
  expect(
    graph.requests.every(
      (request) =>
        Object.isFrozen(request) &&
        Object.isFrozen(request.position) &&
        Object.isFrozen(request.outcome),
    ),
  ).toBe(true)
})

it('reports external, unresolved and ignored boundaries without inventing files', () => {
  // Given requests that legacy traversal does not follow.
  const entry = file(
    'src/main.ts',
    "import 'external'; import './missing'; require('ignored')",
  )
  // When resolving an explicit native false alias.
  const graph = buildStaticImportGraph('src', undefined, {
    cwd: root,
    alias: { ignored: false },
  })
  // Then the three successful observations are distinct.
  expect(graph.requests.map(({ outcome }) => outcome)).toEqual([
    { kind: 'external', request: 'external' },
    { kind: 'unresolved' },
    { kind: 'ignored' },
  ])
  expect(graph.files).toEqual([entry])
  expect(graph.externalImports?.get(entry)).toEqual(new Set(['external']))
})

it.each(['bare', 'absolute', 'nested', 'alias', 'package', 'directory-entry'])(
  'retains the actual matched exclusion entry for %s',
  (mode) => {
    // Given a poisoned target that must never be read or followed.
    const poison = file('blocked/nested/index.ts', "import 'bad'")
    file('node_modules/blocked/package.json', '{')
    const exclusion =
      mode === 'bare' || mode === 'nested' ? 'blocked' : join(root, 'blocked')
    const request =
      mode === 'absolute'
        ? poison.replaceAll('\\', '/')
        : mode === 'alias'
          ? 'provider'
          : mode === 'package'
            ? 'blocked'
            : mode === 'directory-entry'
              ? '../container'
              : '../blocked/nested/index'
    if (mode === 'directory-entry')
      file('container/package.json', '{"main":"../blocked/nested/index.ts"}')
    const excludes =
      mode === 'package' ? [join(root, 'node_modules/blocked')] : [exclusion]
    const entry = file('src/main.ts', `import '${request}'`)
    // When resolving with the existing exclusion matcher.
    const graph = buildStaticImportGraph('src', undefined, {
      cwd: root,
      exclude: excludes,
      alias: { provider: poison },
    })
    // Then evidence names the original matched entry, not a fabricated target.
    expect(graph.requests.map(({ outcome }) => outcome)).toEqual([
      { kind: 'excluded', entry: excludes[0] },
    ])
    expect(graph.files).toEqual([entry])
  },
)

it('orders requests by importer then offset rather than discovery order', () => {
  // Given a followed alias target whose name sorts before the source root.
  const entry = file('src/z.ts', "import 'provider'; import('external')")
  const target = file('outside/a.ts', "require('other')")
  // When traversal discovers the target after the entry.
  const graph = buildStaticImportGraph('src', undefined, {
    cwd: root,
    alias: { provider: target },
  })
  // Then stable evidence ordering is independent of traversal append order.
  expect(
    graph.requests.map(({ importer, position }) => [importer, position.offset]),
  ).toEqual([
    [target, 8],
    [entry, 7],
    [entry, 26],
  ])
})

it('retains an exclusion reached only after canonical package resolution', () => {
  // Given a package symlink whose real target is excluded.
  const manifest = file('blocked/pkg/package.json', '{"main":"index.js"}')
  file('blocked/pkg/index.js', "import 'poison'")
  file('node_modules/placeholder/package.json', '{}')
  fs.symlinkSync(
    dirname(manifest),
    join(root, 'node_modules/linked'),
    'junction',
  )
  const entry = file('src/main.ts', "import 'linked'")
  // When native package resolution canonicalizes the target.
  const graph = buildStaticImportGraph('src', undefined, {
    cwd: root,
    exclude: [dirname(manifest)],
    include: ['linked'],
  })
  // Then the post-resolution exclusion names the actual matcher entry.
  expect(graph.requests.map(({ outcome }) => outcome)).toEqual([
    { kind: 'excluded', entry: dirname(manifest) },
  ])
  expect(graph.files).toEqual([entry])
})

it('keeps the original thrown object untouched and stores only deterministic failing-edge evidence', () => {
  // Given an I/O failure at a real resolution boundary.
  const entry = file('src/main.ts', "import './leaf'")
  const leaf = file('src/leaf.ts')
  const cause = new Error('original cause')
  const original = Object.freeze(new Error('original failure', { cause }))
  const originalStat = fs.statSync
  const stat = spyOn(fs, 'statSync').mockImplementation(
    new Proxy(originalStat, {
      apply(target, thisArg, args) {
        if (args[0] === leaf) throw original
        return Reflect.apply(target, thisArg, args)
      },
    }),
  )
  try {
    const evidence = []
    for (let run = 0; run < 2; run += 1) {
      // When the unchanged resolver throws.
      let caught: unknown
      try {
        buildStaticImportGraph('src', undefined, { cwd: root })
      } catch (error) {
        caught = error
      }
      // Then identity, cause and the failing occurrence are preserved.
      expect(caught).toBe(original)
      expect(original.cause).toBe(cause)
      expect(Object.keys(original)).toEqual([])
      const failure = importGraphFailureOf(caught)
      expect(failure).toEqual([
        {
          importer: entry,
          request: './leaf',
          specifier: './leaf',
          kind: 'static-import',
          position: { offset: 7, line: 1, column: 8 },
          source: 'source',
          outcome: { kind: 'error', error: original },
        },
      ])
      evidence.push(failure)
    }
    expect(evidence[0]).toEqual(evidence[1])
    expect(importGraphFailureOf(new Error('unrelated'))).toBeUndefined()
    expect(importGraphFailureOf(null)).toBeUndefined()
    expect(importGraphFailureOf('primitive')).toBeUndefined()
    expect(importGraphFailureOf(() => undefined)).toBeUndefined()
  } finally {
    stat.mockRestore()
  }
})

it('records alias resolution errors without changing their located diagnostic', () => {
  // Given a terminal alias miss.
  const entry = file('src/main.ts', "export * from 'provider'")
  // When the existing resolver rejects the rewritten candidate.
  let caught: unknown
  try {
    buildStaticImportGraph('src', undefined, {
      cwd: root,
      alias: { provider: './absent' },
    })
  } catch (error) {
    caught = error
  }
  // Then the original error is the same object exposed as evidence.
  expect(caught).toBeInstanceOf(Error)
  expect(importGraphFailureOf(caught)?.[0]).toMatchObject({
    importer: entry,
    kind: 're-export',
    request: 'provider',
    outcome: { kind: 'error', error: caught },
  })
})
