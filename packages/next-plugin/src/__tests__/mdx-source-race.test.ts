import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { createMdxSourceManager } from '../mdx-source-generation'
import { sourceFixture, styledMdx } from './mdx-source-fixture'

it.each(['app/page.mdx', 'provider.tsx'])(
  'rejects when %s changes after real extraction but before delivery',
  async (path) => {
    // Given
    const f = sourceFixture({ 'app/page.mdx': styledMdx })
    const manager = createMdxSourceManager({
      ...f.binding,
      async extractDependencies(view, signal) {
        const reports = await f.binding.extractDependencies(view, signal)
        f.write(path, 'export const changed = true')
        return reports
      },
    })
    // When / Then
    await expect(manager.prepare(f.signal)).rejects.toThrow(join(f.root, path))
  },
)

it('rejects changed ordinary route bytes that are not owned by an MDX importer', async () => {
  // Given
  const f = sourceFixture({
    'app/page.tsx': `import { Box } from '@devup-ui/react'; export default ()=> <Box bg="red" />`,
  })
  const manager = createMdxSourceManager({
    ...f.binding,
    async extractDependencies(view, signal) {
      const reports = await f.binding.extractDependencies(view, signal)
      f.write('app/page.tsx', 'export default ()=>null')
      return reports
    },
  })
  // When / Then
  await expect(manager.prepare(f.signal)).rejects.toThrow(
    join(f.root, 'app/page.tsx'),
  )
})

it('fingerprints and watches ordinary data dependencies without classifying them as compiled MDX', async () => {
  // Given
  const f = sourceFixture({
    'app/page.mdx': styledMdx,
    'df/data.json': '{"color":"blue"}',
  })
  const path = join(f.root, 'df/data.json')
  const manager = createMdxSourceManager({
    ...f.binding,
    async extractDependencies(view, signal) {
      const reports = await f.binding.extractDependencies(view, signal)
      return reports.map((report) =>
        report.filename === 'provider.tsx'
          ? { ...report, dependencies: [path] }
          : report,
      )
    },
  })
  // When
  const generation = await manager.prepare(f.signal)
  // Then
  expect(generation.watchInputs).toContain(path)
  expect(generation.sources).toHaveLength(1)
  expect(
    generation.ordinaryInputs.find((input) => input.filename === 'provider.tsx')
      ?.stamps[path],
  ).toBeDefined()
})

it('keeps directory listings sensitive to names and entry kinds', async () => {
  // Given
  const f = sourceFixture({
    'app/page.mdx': styledMdx,
    'app/reported/a': 'a',
    'app/reported/b': 'b',
  })
  const first = await f.manager.prepare(f.signal)
  f.write('app/reported/c', 'c')
  // When
  const next = await f.manager.refresh({ generation: first, signal: f.signal })
  // Then
  expect(f.counts()).toBe(2)
  expect(
    next.compiled[join(f.root, 'app/page.mdx')]?.prepared.source,
  ).toContain('a,b,c')
})
