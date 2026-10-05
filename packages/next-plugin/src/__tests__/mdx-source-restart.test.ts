import { mkdirSync, utimesSync } from 'node:fs'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { isMdxRecord } from '../mdx-pipeline'
import { importMdxRestartCache } from '../mdx-source-cache'
import { createMdxSourceManager } from '../mdx-source-generation'
import { sourceFixture, styledMdx } from './mdx-source-fixture'

it.each(['resource', 'file', 'context', 'missing'] as const)(
  'invalidates persisted compiled bytes when the %s input changes',
  async (kind) => {
    // Given
    const f = sourceFixture({
      'app/page.mdx': styledMdx,
      'app/data.json': '{"color":"blue"}',
    })
    const directory = join(f.root, 'app/reported')
    mkdirSync(directory)
    const first = await f.manager.prepare(f.signal)
    const saved = f.manager.restartCache(first)
    switch (kind) {
      case 'resource':
        f.write('app/page.mdx', styledMdx.replace('red', 'yellow'))
        break
      case 'file':
        f.write('app/data.json', '{"color":"green"}')
        break
      case 'context':
        f.write('app/reported/added', 'entry')
        break
      case 'missing':
        f.write('app/optional.json', '{}')
        break
      default:
        kind satisfies never
    }
    // When
    await createMdxSourceManager(f.binding, saved).prepare(f.signal)
    // Then
    expect(f.counts()).toBe(2)
  },
)

it('refreshes an imported ordinary constant after cache restart even outside graph discovery', async () => {
  // Given
  const f = sourceFixture({
    'app/page.mdx': `import { css } from '@devup-ui/react'\n\nimport { color } from '../df/value'\n\nexport const style = css({color})\n\n# Test`,
    'df/value.ts': "export const color = 'blue'",
  })
  const first = await f.manager.prepare(f.signal)
  const saved = f.manager.restartCache(first)
  f.write('df/value.ts', "export const color = 'green'")
  // When
  const next = await createMdxSourceManager(f.binding, saved).prepare(f.signal)
  // Then
  expect(f.counts()).toBe(2)
  expect(f.css(next)).toContain('color:green')
})

it('reprepares MDX when a compiler-injected provider changes, including after restart', async () => {
  // Given
  const f = sourceFixture({ 'app/page.mdx': styledMdx }, {}, true)
  const first = await f.manager.prepare(f.signal)
  const saved = f.manager.restartCache(first)
  f.write(
    'provider.tsx',
    `import { Box } from '@devup-ui/react'; export function useMDXComponents() { return { h1: () => <Box p={5}/> } }`,
  )
  const refreshed = await f.manager.refresh({
    generation: first,
    signal: f.signal,
  })
  expect(f.counts()).toBe(2)
  f.write(
    'provider.tsx',
    `import { Box } from '@devup-ui/react'; export function useMDXComponents() { return { h1: () => <Box p={6}/> } }`,
  )
  // When
  const next = await createMdxSourceManager(f.binding, saved).prepare(f.signal)
  // Then
  expect(f.counts()).toBe(3)
  expect(f.css(next)).toContain('padding:24px')
  expect(f.css(refreshed)).toContain('padding:20px')
})

it('forces re-preparation on directory child watch events even when entry names stay unchanged', async () => {
  // Given
  const f = sourceFixture(
    { 'app/page.mdx': styledMdx, 'app/reported/existing': 'before' },
    {},
    true,
  )
  const directory = join(f.root, 'app/reported')
  utimesSync(
    directory,
    new Date(1_700_000_000_000),
    new Date(1_700_000_000_000),
  )
  const first = await f.manager.prepare(f.signal)
  const path = f.write('app/reported/existing', 'after')
  // When
  await f.manager.refresh({
    generation: first,
    signal: f.signal,
    changedPaths: [path],
  })
  // Then
  expect(f.counts()).toBe(2)
})

it('prunes old compiled MDX from the current-plan reader when its import is dropped', async () => {
  // Given
  const f = sourceFixture({
    'app/page.tsx': `import './child.mdx'; export default ()=>null`,
    'app/child.mdx': styledMdx,
  })
  const first = await f.manager.prepare(f.signal)
  f.write('app/page.tsx', 'export default ()=>null')
  // When
  const next = await f.manager.refresh({ generation: first, signal: f.signal })
  // Then
  expect(next.sources).toEqual([])
  expect(next.cacheReader(join(f.root, 'app/child.mdx'))).toBeUndefined()
  expect(first.cacheReader(join(f.root, 'app/child.mdx'))).toBeDefined()
})

it('reprepares compiled importers when another reached MDX resource changes', async () => {
  // Given
  const f = sourceFixture(
    {
      'app/page.mdx': `import './child.mdx'\n\n${styledMdx}`,
      'app/child.mdx': styledMdx,
    },
    {},
    true,
  )
  const first = await f.manager.prepare(f.signal)
  f.write('app/child.mdx', styledMdx.replace('red', 'blue'))
  // When
  await f.manager.refresh({ generation: first, signal: f.signal })
  // Then
  expect(f.counts()).toBe(4)
})

it('rejects a partial persisted proof even when its pipeline and compiled source are intact', async () => {
  // Given
  const f = sourceFixture({ 'app/page.mdx': styledMdx })
  const first = await f.manager.prepare(f.signal)
  const data: unknown = JSON.parse(f.manager.restartCache(first))
  if (!Array.isArray(data)) throw new TypeError('Missing restart fixture')
  const entry: unknown = data[0]
  if (!isMdxRecord(entry)) throw new TypeError('Missing restart entry')
  // When / Then
  expect(() =>
    importMdxRestartCache(JSON.stringify([{ ...entry, inputs: [] }])),
  ).toThrow('Incomplete MDX restart input proof')
})
