import { join } from 'node:path'

import { createModuleResolver } from '@devup-ui/plugin-utils'
import { expect, it } from 'bun:test'

import { createMdxSourceManager } from '../mdx-source-generation'
import { sourceFixture, styledMdx } from './mdx-source-fixture'

it('prepares only reached MDX when an unreachable compiler-invalid file exists', async () => {
  // Given
  const f = sourceFixture({
    'app/page.mdx': styledMdx,
    'dead.mdx': '<Box',
    'app/data.json': '{"color":"purple"}',
  })
  // When
  const generation = await f.manager.prepare(f.signal)
  // Then
  expect(f.counts()).toBe(1)
  expect(Object.keys(generation.compiled)).toEqual([
    join(f.root, 'app/page.mdx'),
  ])
  expect(
    generation.compiled[join(f.root, 'app/page.mdx')]?.prepared.map,
  ).toBeDefined()
  expect(f.css(generation)).toContain('color:purple')
  expect(generation.cacheReader(join(f.root, 'dead.mdx'))).toBeUndefined()
  expect(generation.resolver.prepareSource).toBe(generation.cacheReader)
  expect(generation.plan.seedFiles).toContain('app/page.mdx')
})

it('fails real invalid MDX when prewarmAll is explicitly requested', async () => {
  // Given
  const f = sourceFixture({ 'app/page.mdx': styledMdx, 'dead.mdx': '<Box' })
  const manager = createMdxSourceManager({
    ...f.binding,
    effectiveAppContext: { ...f.binding.effectiveAppContext, prewarmAll: true },
  })
  // When / Then
  await expect(manager.prepare(f.signal)).rejects.toThrow(
    join(f.root, 'dead.mdx'),
  )
})

it('numbers styles injected into Markdown that contains no raw styling import', async () => {
  // Given
  const f = sourceFixture({
    'app/page.mdx': '# No source styles',
    'app/data.json': '{"color":"orange"}',
  })
  // When
  const generation = await f.manager.prepare(f.signal)
  // Then
  expect(generation.plan.seedFiles).toContain('app/page.mdx')
  expect(f.css(generation)).toContain('color:orange')
})

it('closes compiler-injected provider and MDX edges through shared graph iterations', async () => {
  // Given
  const f = sourceFixture({
    'app/page.mdx': styledMdx,
    'nested.mdx': styledMdx,
  })
  f.write(
    'provider.tsx',
    `import './nested.mdx'; import { Box } from '@devup-ui/react'; export function useMDXComponents() { return { h1: () => <Box p={3} /> } }`,
  )
  // When
  const generation = await f.manager.prepare(f.signal)
  // Then
  expect(f.counts()).toBe(2)
  expect(generation.plan.expectedBaseFiles).toEqual(
    expect.arrayContaining(['app/page.mdx', 'nested.mdx', 'provider.tsx']),
  )
  expect(f.css(generation)).toContain('padding:12px')
})

it('numbers compiler-injected style sources and preserves ordinary imports under output basenames', async () => {
  // Given
  const f = sourceFixture({
    'app/page.mdx': `import { css } from '@devup-ui/react'\n\nimport { value } from '../src/build/value'\n\nexport const style = css({background: value})\n\n# Test`,
    'app/data.json': '{"color":"green"}',
    'src/build/value.ts': "export const value = 'yellow'",
    'src/df/source.ts': 'export const source = 1',
    'df/broken.mdx': '<Box',
    '.next/broken.mdx': '<Box',
  })
  // When
  const generation = await f.manager.prepare(f.signal)
  // Then
  expect(
    generation.plan.graph.fileSet.has(join(f.root, 'src/df/source.ts')),
  ).toBe(true)
  expect(generation.plan.graph.fileSet.has(join(f.root, 'df/broken.mdx'))).toBe(
    false,
  )
  expect(generation.plan.seedFiles).toContain('app/page.mdx')
  expect(f.css(generation)).toContain('background:yellow')
})

it('uses compiled exports under original IDs when an ordinary route imports Markdown', async () => {
  // Given
  const f = sourceFixture({
    'app/page.tsx': `import { css } from '@devup-ui/react'; import { color } from './value.md'; export const style = css({color})`,
    'app/value.md': `export const color = 'blue'\n\n# Value`,
  })
  f.compilerOptions.jsx = false
  // When
  const generation = await f.manager.prepare(f.signal)
  // Then
  expect(generation.sources[0]?.input.filename).toBe('app/value.md')
  expect(f.css(generation)).toContain('color:blue')
})

it('imports JSX-preserving compiled Markdown under its original identity without changing native options', async () => {
  // Given
  const f = sourceFixture({
    'app/page.tsx': `import { css } from '@devup-ui/react'; import { color } from './value.md'; export const style = css({color})`,
    'app/value.md': `export const color = 'blue'\n\n# Value`,
  })
  // When
  const generation = await f.manager.prepare(f.signal)
  // Then
  expect(f.compilerOptions.jsx).toBe(true)
  expect(generation.sources[0]?.input.filename).toBe('app/value.md')
  expect(generation.sources[0]?.input.source).toMatch(/<[_A-Za-z]/)
  expect(f.css(generation)).toContain('color:blue')
})

it('returns native-only ordinary expectations without waiting for future loader bytes', async () => {
  // Given
  const f = sourceFixture({ 'app/page.mdx': styledMdx })
  const proof = {}
  const manager = createMdxSourceManager({
    ...f.binding,
    ordinaryEligibility: (filename) =>
      filename === join(f.root, 'provider.tsx')
        ? {
            kind: 'native-required',
            expectation: { filename, reason: 'upstream-loader', proof },
          }
        : { kind: 'disk-first' },
  })
  // When
  const generation = await manager.prepare(f.signal)
  // Then
  expect(generation.pendingOrdinary).toHaveLength(1)
  expect(
    generation.ordinaryInputs.some(
      (input) => input.filename === 'provider.tsx',
    ),
  ).toBe(false)
  expect(f.extractionCalls()).toBe(0)
  expect(generation.sources).toHaveLength(1)
  expect(() => manager.validateForCssFinalization(generation)).toThrow(
    'genuine pre-Devup loader input',
  )
})

it('lets the shared resolver locate unmatched-rule Markdown rather than reading it raw', async () => {
  // Given
  const f = sourceFixture(
    {
      'app/page.tsx': `import { color } from './value.mdx'; export default ()=>color`,
      'app/value.mdx': `export const color = 'blue'`,
    },
    { selectPipeline: async () => undefined },
  )
  // When / Then
  await expect(f.manager.prepare(f.signal)).rejects.toThrow(
    `${join(f.root, 'app/page.tsx')}:1:1: module source preparation`,
  )
})

it('never serves excluded compiled records while ordinary output dependencies still resolve', async () => {
  // Given
  const f = sourceFixture({
    'app/page.mdx': styledMdx,
    'df/value.ts': 'export const value = 4',
    'df/value.mdx': 'export const value = 9',
  })
  const generation = await f.manager.prepare(f.signal)
  const resolver = createModuleResolver({ cwd: f.root, ...generation.resolver })
  // When / Then
  expect(resolver('../df/value.ts', 'app/page.mdx')?.code).toContain('4')
  expect(() => resolver('../df/value.mdx', 'app/page.mdx')).toThrow(
    'Markdown source has no prepared JavaScript',
  )
})

it('rejects other configured extensions until the explicit source type API is merged', () => {
  // Given
  const f = sourceFixture({})
  // When / Then
  expect(() =>
    createMdxSourceManager({ ...f.binding, extensions: ['.mdown'] }),
  ).toThrow(`${f.binding.configFile}:1:1`)
})
