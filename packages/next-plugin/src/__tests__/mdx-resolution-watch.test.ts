import { dirname, join } from 'node:path'

import { expect, it } from 'bun:test'

import { watchSources } from '../coordinator-watch'
import { createMdxSourceManager } from '../mdx-source-generation'
import { paletteFixture, paletteMdx } from './mdx-resolution-fixture'
import { sourceFixture, styledMdx } from './mdx-source-fixture'

it.each(['manifest', 'missing'] as const)(
  'refreshes CSS after an actual adopted %s watch event',
  async (kind) => {
    // Given
    const f = paletteFixture()
    const earlier = join(f.root, 'outside/earlier.js')
    const manager =
      kind === 'manifest'
        ? f.manager
        : createMdxSourceManager({
            ...f.binding,
            aliases: {
              ...f.binding.aliases,
              palette$: [earlier, join(f.root, 'node_modules/palette/red.js')],
            },
          })
    const first = await manager.prepare(f.signal)
    const target = kind === 'manifest' ? f.manifest : earlier
    expect(first.watchInputs).toContain(target)
    const event = Promise.withResolvers<readonly string[]>()
    const watcher = watchSources({
      roots: [],
      debounceMs: 5,
      onChange: (paths) => {
        if (paths?.some((path) => path === target || path === dirname(target)))
          event.resolve(paths)
      },
      onError: (error) => event.reject(error),
    })
    watcher.replaceInputs?.(first.watchInputs)
    const timer = setTimeout(
      () =>
        event.reject(
          new Error('Resolution watch did not deliver the changed input'),
        ),
      3000,
    )
    try {
      // When
      if (kind === 'manifest') f.select('blue')
      else f.write('outside/earlier.js', 'export const color = "blue"')
      const changedPaths = await event.promise
      const next = await manager.refresh({
        generation: first,
        signal: f.signal,
        changedPaths,
      })
      // Then
      expect(next).not.toBe(first)
      expect(f.counts()).toBe(2)
      expect(f.css(next)).toContain('color:blue')
      expect(f.css(next)).not.toContain('color:red')
    } finally {
      clearTimeout(timer)
      watcher.close()
    }
  },
)

it('invalidates ordinary styling metadata with zero compiled MDX inputs', async () => {
  // Given
  const f = paletteFixture({
    'app/page.tsx': `import { css } from '@devup-ui/react'; import { color } from 'palette'; export const style = css({color}); export default function Page(){return <div/>}`,
  })
  const first = await f.manager.prepare(f.signal)
  expect(first.sources).toEqual([])
  f.select('blue')
  // When
  const next = await f.manager.refresh({ generation: first, signal: f.signal })
  // Then
  expect(next).not.toBe(first)
  expect(f.css(next)).toContain('color:blue')
  expect(f.counts()).toBe(0)
})

it('rejects a mid-observation manifest race without mutating predecessor proof', async () => {
  // Given
  const f = paletteFixture()
  let racing = false
  const manager = createMdxSourceManager({
    ...f.binding,
    async extractDependencies(view, signal) {
      const reports = await f.binding.extractDependencies(view, signal)
      if (racing) f.select('blue')
      return reports
    },
  })
  const first = await manager.prepare(f.signal)
  const proof = JSON.stringify(first.resolutionInputs)
  f.write('app/page.mdx', paletteMdx + '\n\nChanged')
  racing = true
  // When / Then
  await expect(
    manager.refresh({ generation: first, signal: f.signal }),
  ).rejects.toThrow(f.manifest)
  expect(JSON.stringify(first.resolutionInputs)).toBe(proof)
  expect(first.sources[0]?.input.source).not.toContain('Changed')
})

it('rejects a compiler-only dependency change during extraction independently of resolution reports', async () => {
  // Given
  const f = sourceFixture({
    'app/page.mdx': styledMdx,
    'app/data.json': '{"color":"red"}',
  })
  const manager = createMdxSourceManager({
    ...f.binding,
    async extractDependencies(view, signal) {
      const reports = await f.binding.extractDependencies(view, signal)
      f.write('app/data.json', '{"color":"blue"}')
      return reports
    },
  })
  // When / Then
  await expect(manager.prepare(f.signal)).rejects.toThrow(
    join(f.root, 'app/data.json'),
  )
})

it('rejects a newly reported ordinary dependency race when an extraction provider supplies no resolver proof', async () => {
  // Given
  const f = sourceFixture({
    'app/page.mdx': styledMdx,
    'df/plain.ts': 'export const value = 1',
  })
  let reports = 0
  const manager = createMdxSourceManager({
    ...f.binding,
    async extractDependencies() {
      reports += 1
      if (reports === 2) f.write('df/plain.ts', 'export const value = 2')
      return [{ filename: 'app/page.mdx', dependencies: ['df/plain.ts'] }]
    },
  })
  // When / Then
  await expect(manager.prepare(f.signal)).rejects.toThrow(
    'ordinary input changed during extraction',
  )
})
