import { rmSync } from 'node:fs'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { createMdxSourceManager } from '../mdx-source-generation'
import { sourceFixture } from './mdx-source-fixture'

it.each(['addition', 'removal'] as const)(
  'replaces the numbering plan when an unreachable styled source changes by %s',
  async (change) => {
    // Given
    const source = `import { Box } from '@devup-ui/react'; export const card = <Box p={2} />`
    const f = sourceFixture({
      'app/page.tsx': 'export default () => null',
      ...(change === 'removal' ? { 'unused.tsx': source } : {}),
    })
    const first = await f.manager.prepare(f.signal)
    switch (change) {
      case 'addition':
        f.write('unused.tsx', source)
        break
      case 'removal':
        rmSync(join(f.root, 'unused.tsx'))
        break
      default:
        change satisfies never
    }
    // When
    const next = await f.manager.refresh({
      generation: first,
      signal: f.signal,
    })
    // Then
    expect(next.plan.seedFiles.includes('unused.tsx')).toBe(
      change === 'addition',
    )
    expect(next).not.toBe(first)
    expect(next.plan.expectedBaseFiles).toEqual(first.plan.expectedBaseFiles)
    expect(Object.isFrozen(next.plan.seedFiles)).toBe(true)
  },
)

it('replaces canonical buckets when an unreachable importer changes collapse without changing route inputs', async () => {
  // Given
  const f = sourceFixture({
    'app/page.tsx': `import { value } from '../shared'; export default () => value`,
    'shared.ts': 'export const value = 1',
  })
  const first = await f.manager.prepare(f.signal)
  f.write('unused.ts', `import { value } from './shared'; export { value }`)
  // When
  const next = await f.manager.refresh({ generation: first, signal: f.signal })
  // Then
  expect(next.plan.canonicalMap).not.toEqual(first.plan.canonicalMap)
  expect(next.plan.expectedBaseFiles).toEqual(first.plan.expectedBaseFiles)
  expect(next).not.toBe(first)
})

it.each(['routes', 'threshold'] as const)(
  'replaces the hoist plan when its %s change without changing source bytes',
  async (field) => {
    // Given
    const f = sourceFixture({
      'app/a/page.tsx': `import '../../shared'; export default () => null`,
      'app/b/page.tsx': `import '../../shared'; export default () => null`,
      'shared.tsx': `import { Box } from '@devup-ui/react'; export const card = <Box p={2} />`,
    })
    let threshold = field === 'routes' ? undefined : 2
    const manager = createMdxSourceManager({
      ...f.binding,
      effectiveAppContext: {
        ...f.binding.effectiveAppContext,
        get atomHoist() {
          return threshold
        },
      },
    })
    const first = await manager.prepare(f.signal)
    threshold = field === 'routes' ? 2 : 3
    // When
    const next = await manager.refresh({ generation: first, signal: f.signal })
    // Then
    expect(next.plan.atomThreshold).toBe(threshold)
    expect(next).not.toBe(first)
    expect(next.plan.expectedBaseFiles).toEqual(first.plan.expectedBaseFiles)
    expect(next.plan.fileRoutes['shared.tsx']).toEqual([0, 1])
  },
)
