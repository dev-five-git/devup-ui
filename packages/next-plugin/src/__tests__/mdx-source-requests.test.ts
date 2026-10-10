import { join } from 'node:path'

import { buildStaticImportGraph } from '@devup-ui/plugin-utils'
import { expect, it } from 'bun:test'

import { createMdxSourceManager } from '../mdx-source-generation'
import { immutableMdxPlan } from '../mdx-source-immutable'
import { expectedJoinInputs } from './join-resolution-inputs'
import { sourceFixture, styledMdx } from './mdx-source-fixture'

it('retains request correlations when real generations swap the same target set', async () => {
  // Given
  const f = sourceFixture({
    'app/red.ts': "export const value = 'red'",
    'app/blue.ts': "export const value = 'blue'",
  })
  const source = (first: string, second: string) =>
    `import { css } from '@devup-ui/react'; import {value as a} from '${first}'; import {value as b} from '${second}'; import '${first}'; export const cls = css({color:a, background:b})`
  const page = f.write('app/page.tsx', source('./red', './blue'))
  const before = await f.manager.prepare(f.signal)
  f.write('app/page.tsx', source('./blue', './red'))
  // When
  const after = await f.manager.prepare(f.signal)
  // Then
  expect([...(after.plan.graph.staticImports.get(page) ?? [])].sort()).toEqual(
    [...(before.plan.graph.staticImports.get(page) ?? [])].sort(),
  )
  expect(
    before.plan.graph.requests
      ?.filter((edge) => edge.importer === page)
      .map((edge) => edge.specifier),
  ).toEqual(['@devup-ui/react', './red', './blue', './red'])
  expect(
    after.plan.graph.requests
      ?.filter((edge) => edge.importer === page)
      .map((edge) => edge.specifier),
  ).toEqual(['@devup-ui/react', './blue', './red', './blue'])
  expect(f.css(before)).toContain('color:red')
  expect(f.css(after)).toContain('color:blue')
})

it('transports the exact compiled producer ledger and map through immutable delivery', async () => {
  // Given
  const f = sourceFixture({
    'app/page.mdx': `import './leaf'\n\n${styledMdx}`,
    'app/leaf.ts': 'export {}',
  })
  const generation = await f.manager.prepare(f.signal)
  const graph = await buildStaticImportGraph(f.root, undefined, {
    cwd: f.root,
    exclude: ['df', '.next'],
    includeMdx: f.binding.extensions,
    alias: f.binding.aliases,
    conditions: f.binding.conditions,
    prepareSource: generation.cacheReader,
  })
  // When
  const plan = immutableMdxPlan(
    {
      ...generation.plan,
      graph,
      seedFiles: [...generation.plan.seedFiles],
      expectedBaseFiles: [...generation.plan.expectedBaseFiles],
      canonicalMap: { ...generation.plan.canonicalMap },
      fileRoutes: {},
    },
    graph,
  )
  // Then
  expect(plan.graph.requests).toBe(graph.requests)
  expect(generation.plan.graph.requests).toEqual(graph.requests)
  const edge = plan.graph.requests?.find(
    (request) =>
      request.importer === join(f.root, 'app/page.mdx') &&
      request.specifier === './leaf',
  )
  expect(edge).toMatchObject({
    source: 'compiled',
    kind: 'static-import',
    request: './leaf',
    outcome: {
      kind: 'resolved',
      path: join(f.root, 'app/leaf.ts'),
      inputs: expectedJoinInputs(
        f.root,
        [join(f.root, 'app/leaf.ts')],
        ['app/leaf'],
      ),
    },
  })
  expect(edge?.map).toBe(
    generation.compiled[join(f.root, 'app/page.mdx')]?.prepared.map,
  )
})

it('retains absent legacy evidence instead of inventing an empty scanned ledger', () => {
  // Given
  const graph = {
    files: [],
    fileSet: new Set<string>(),
    staticImports: new Map<string, Set<string>>(),
    staticImporters: new Map<string, Set<string>>(),
    dynamicImports: new Map<string, Set<string>>(),
    dynamicTargets: new Set<string>(),
  }
  const sourcePlan = {
    graph,
    seedFiles: [],
    expectedBaseFiles: [],
    canonicalMap: {},
    fileRoutes: {},
    atomThreshold: null,
  }
  // When
  const plan = immutableMdxPlan(sourcePlan, graph)
  // Then
  expect(Object.hasOwn(plan.graph, 'requests')).toBe(false)
  expect(plan.graph.requests).toBeUndefined()
})

it('preserves raw spelling, decoded request and duplicate source coordinates', async () => {
  // Given
  const source = String.raw`import './\x6ceaf'; export * from './leaf'; import('./leaf'); require('./leaf');`
  const f = sourceFixture({
    'app/page.tsx': source,
    'app/leaf.ts': 'export {}',
  })
  // When
  const generation = await f.manager.prepare(f.signal)
  // Then
  expect(
    generation.plan.graph.requests?.filter(
      (edge) => edge.importer === join(f.root, 'app/page.tsx'),
    ),
  ).toEqual(
    (
      [
        ['static-import', './\\x6ceaf', 7],
        ['re-export', './leaf', 34],
        ['literal-dynamic-import', './leaf', 51],
        ['literal-require', './leaf', 70],
      ] as const
    ).map(([kind, request, offset]) => ({
      importer: join(f.root, 'app/page.tsx'),
      kind,
      request,
      specifier: './leaf',
      position: { offset, line: 1, column: offset + 1 },
      source: 'source',
      outcome: {
        kind: 'resolved',
        path: join(f.root, 'app/leaf.ts'),
        inputs: expectedJoinInputs(
          f.root,
          [join(f.root, 'app/leaf.ts')],
          ['app/leaf'],
        ),
      },
    })),
  )
})

it('delivers descriptor aliases through prepared graph and WASM under original filenames', async () => {
  // Given
  const f = sourceFixture({
    'app/page.mdx':
      "import {css} from '@devup-ui/react'\n\nimport {value} from 'value'\n\nexport const cls = css({color:value})\n\n# Heading",
  })
  const value = f.write('outside/value.ts', "export const value = 'blue'")
  const manager = createMdxSourceManager({
    ...f.binding,
    aliases: [
      {
        name: 'provider',
        alias: join(f.root, 'provider.tsx'),
        onlyModule: true,
      },
      { name: 'value', alias: [join(f.root, 'absent'), value] },
      { name: 'value', alias: false },
    ],
  })
  // When
  const generation = await manager.prepare(f.signal)
  // Then
  expect(generation.plan.graph.fileSet.has(value)).toBe(true)
  expect(generation.resolver.prepareSource).toBe(generation.cacheReader)
  expect(
    generation.plan.graph.requests?.find((edge) => edge.specifier === 'value')
      ?.outcome,
  ).toEqual({
    kind: 'resolved',
    path: value,
    inputs: expectedJoinInputs(
      f.root,
      [value],
      ['absent', 'absent/package.json', 'absent/index.ts'],
    ),
  })
  expect(f.css(generation)).toContain('color:blue')
})

it('retains distinct non-file outcomes when the producer skips graph targets', async () => {
  // Given
  const f = sourceFixture(
    {
      'app/page.tsx':
        "import './missing'; import 'external'; import 'ignored'; import '../df/blocked';",
      'df/blocked.ts': 'invalid unvisited output',
    },
    { aliases: { ignored: false } },
  )
  // When
  const generation = await f.manager.prepare(f.signal)
  // Then
  expect(
    generation.plan.graph.requests
      ?.filter((edge) => edge.importer === join(f.root, 'app/page.tsx'))
      .map((edge) => edge.outcome),
  ).toEqual([
    { kind: 'unresolved' },
    { kind: 'external', request: 'external' },
    { kind: 'ignored' },
    { kind: 'excluded', entry: join(f.root, 'df') },
  ])
})
