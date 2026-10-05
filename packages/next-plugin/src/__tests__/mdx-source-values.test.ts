import { join } from 'node:path'

import { expect, it } from 'bun:test'

import {
  captureMdxCacheIdentity,
  compatibleMdxCache,
  exportMdxRestartCache,
  importMdxRestartCache,
} from '../mdx-source-cache'
import { createMdxTimestampAccuracy } from '../mdx-source-freshness'
import { createMdxSourceManager } from '../mdx-source-generation'
import { immutableMdxMap } from '../mdx-source-immutable'
import { overlayMdxOrdinaryInputs } from '../mdx-source-inputs'
import { sourceFixture, styledMdx } from './mdx-source-fixture'

it.each([
  [2000, 2000],
  [1000, 1000],
  [100, 100],
  [10, 10],
  [1, 1],
])(
  'narrows installed-webpack timestamp accuracy for timestamp %s',
  (mtime, margin) => {
    // Given / When / Then
    expect(createMdxTimestampAccuracy().apply(mtime)).toBe(margin)
  },
)

it('provides immutable graph query views without publishing mutable Map or Set instances', async () => {
  // Given
  const f = sourceFixture({ 'app/page.mdx': styledMdx })
  const generation = await f.manager.prepare(f.signal)
  const graph = generation.plan.graph
  // When / Then
  expect('set' in graph.staticImports).toBe(false)
  expect('add' in graph.fileSet).toBe(false)
  expect(graph.fileSet.size).toBe(graph.files.length)
  expect([...graph.fileSet]).toHaveLength(graph.files.length)
  expect([...graph.fileSet.keys()]).toEqual([...graph.fileSet.values()])
  expect([...graph.fileSet.entries()]).toHaveLength(graph.fileSet.size)
  const visited: string[] = []
  graph.fileSet.forEach((file) => visited.push(file))
  expect(visited).toHaveLength(graph.fileSet.size)
  expect(graph.fileSet.union(new Set(['extra'])).has('extra')).toBe(true)
  expect(graph.fileSet.intersection(new Set(['extra'])).size).toBe(0)
  expect(graph.fileSet.difference(new Set()).size).toBe(graph.fileSet.size)
  expect(
    graph.fileSet.symmetricDifference(new Set(['extra'])).has('extra'),
  ).toBe(true)
  expect(graph.fileSet.isSubsetOf(new Set(graph.files))).toBe(true)
  expect(graph.fileSet.isSupersetOf(new Set())).toBe(true)
  expect(graph.fileSet.isDisjointFrom(new Set(['extra']))).toBe(true)
  expect(graph.staticImports.size).toBe(graph.files.length)
  expect(graph.staticImports.has(graph.files[0] ?? '')).toBe(true)
  expect(
    graph.staticImports
      .get(join(f.root, 'app/page.mdx'))
      ?.has(join(f.root, 'provider.tsx')),
  ).toBe(true)
  expect([...graph.staticImports.keys()]).toHaveLength(graph.staticImports.size)
  expect([...graph.staticImports.values()]).toHaveLength(
    graph.staticImports.size,
  )
  let count = 0
  graph.staticImports.forEach(() => {
    count += 1
  })
  expect(count).toBe(graph.staticImports.size)
  expect([...graph.staticImports]).toHaveLength(graph.staticImports.size)
  expect([...graph.staticImports.entries()]).toHaveLength(
    graph.staticImports.size,
  )
  expect(
    Object.isFrozen(
      generation.compiled[join(f.root, 'app/page.mdx')]?.prepared.map,
    ),
  ).toBe(true)
})

it('copies compiler maps without decoding them or freezing the producer object', () => {
  // Given
  const map = { version: 3, sources: ['original.mdx'], nested: { empty: null } }
  // When
  const copy = immutableMdxMap(map)
  // Then
  expect(copy).toEqual(map)
  expect(Object.isFrozen(map)).toBe(false)
  expect(immutableMdxMap('map')).toBe('map')
  expect(immutableMdxMap(undefined)).toBeUndefined()
})

it('freezes route-hoist arrays from the real shared prepared graph', async () => {
  // Given
  const f = sourceFixture({
    'app/a/page.mdx': `import '../../shared'\n\n${styledMdx}`,
    'app/b/page.mdx': `import '../../shared'\n\n${styledMdx}`,
    'shared.tsx': `import { Box } from '@devup-ui/react'; export const Shared = ()=> <Box p={2}/>`,
  })
  const manager = createMdxSourceManager({
    ...f.binding,
    effectiveAppContext: { ...f.binding.effectiveAppContext, atomHoist: 2 },
  })
  // When
  const generation = await manager.prepare(f.signal)
  // Then
  expect(generation.plan.fileRoutes['shared.tsx']).toEqual([0, 1])
  expect(Object.isFrozen(generation.plan.fileRoutes['shared.tsx'])).toBe(true)
})

it('overlays additions on an ordinary ledger without MDX-specific reach pruning', async () => {
  // Given
  const f = sourceFixture({ 'app/page.mdx': styledMdx })
  const generation = await f.manager.prepare(f.signal)
  const provider = generation.ordinaryInputs.find(
    (input) => input.filename === 'provider.tsx',
  )
  if (!provider) throw new TypeError('Missing ordinary provider')
  // When
  const inputs = overlayMdxOrdinaryInputs(generation.ordinaryInputs, [
    { ...provider, source: 'replacement' },
  ])
  // Then
  expect(
    inputs.find((input) => input.filename === provider.filename)?.source,
  ).toBe('replacement')
  expect(inputs).toHaveLength(generation.ordinaryInputs.length)
  expect(overlayMdxOrdinaryInputs(inputs, [])).toHaveLength(inputs.length)
})

it.each([
  () => {},
  new Date(),
  Symbol('leaf'),
  NaN,
  { value: undefined },
  Object.defineProperty({}, 'getter', { get: () => 1 }),
  new Array(2),
])(
  'invalidates nonportable identity across restart without executing getters',
  async (identity) => {
    // Given
    const f = sourceFixture({})
    const selection = {
      pipeline: f.pipeline,
      context: { owner: {}, generation: {} },
      identity,
    }
    // When
    const captured = captureMdxCacheIdentity(selection)
    // Then
    expect(captured.portable).toBeUndefined()
    expect(
      exportMdxRestartCache(
        new Map([
          [
            'module',
            {
              ...captured,
              prepared: {
                filename: 'module',
                source: '',
                map: undefined,
                dependencies: [],
                contextDependencies: [],
                missingDependencies: [],
                buildDependencies: [],
                dependencyReports: [],
              },
              inputs: [],
            },
          ],
        ]),
      ),
    ).toBe('[]')
  },
)

it('preserves structural identity while separating map delivery cache dimensions', async () => {
  // Given
  const f = sourceFixture({ 'app/page.mdx': styledMdx })
  const selection = await f.binding.selectPipeline('module', f.signal)
  if (!selection) throw new TypeError('Missing selected pipeline')
  const captured = captureMdxCacheIdentity(selection)
  const entry = {
    ...captured,
    prepared: {
      filename: 'module',
      source: '',
      map: undefined,
      dependencies: [],
      contextDependencies: [],
      missingDependencies: [],
      buildDependencies: [],
      dependencyReports: [],
    },
    inputs: [],
  }
  // When / Then
  expect(
    compatibleMdxCache(entry, {
      ...selection,
      identity: structuredClone(selection.identity),
    }),
  ).toBe(true)
  expect(
    compatibleMdxCache(entry, {
      ...selection,
      context: { ...selection.context, sourceMap: false },
    }),
  ).toBe(false)
  expect(
    compatibleMdxCache(entry, { ...selection, identity: ['changed'] }),
  ).toBe(false)
})

it.each([
  '{}',
  '[{}]',
  '[{"filename":"m","prepared":{"filename":"m","source":9},"inputs":[]}]',
  '[{"filename":"m","identity":{},"prepared":{"filename":"m","source":"","dependencies":[9]},"inputs":[]}]',
  '[{"filename":"m","identity":{},"prepared":{"filename":"m","source":""},"inputs":[{}]}]',
])('rejects malformed private restart data when payload is %s', (payload) => {
  // Given / When / Then
  expect(() => importMdxRestartCache(payload)).toThrow()
})
