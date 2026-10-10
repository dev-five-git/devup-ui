import { expect, it } from 'bun:test'

import { immutableGeneration } from '../coordinator-generation'
import { sourceFixture, styledMdx } from './mdx-source-fixture'

it('returns the supplied Core copy unchanged when all generation inputs are fresh', async () => {
  // Given
  const f = sourceFixture({ 'app/page.mdx': styledMdx })
  const original = await f.manager.prepare(f.signal)
  const supplied = immutableGeneration(original)
  // When
  const next = await f.manager.refresh({
    generation: supplied,
    signal: f.signal,
  })
  // Then
  expect(next).toBe(supplied)
  expect(f.counts()).toBe(1)
})

it('refreshes a Core copy to a complete new generation through the existing public replay type', async () => {
  // Given
  const f = sourceFixture({ 'app/page.mdx': styledMdx })
  const original = await f.manager.prepare(f.signal)
  const supplied = immutableGeneration(original)
  f.write('app/page.mdx', styledMdx.replace('red', 'blue'))
  // When
  const next = await f.manager.refresh({
    generation: supplied,
    signal: f.signal,
  })
  // Then
  expect(next.sources[0]?.input.source).toContain('blue')
  expect(f.counts()).toBe(2)
})

it('rejects changed source bytes even when a supplied copy reuses the owned configurer', async () => {
  // Given
  const f = sourceFixture({ 'app/page.mdx': styledMdx })
  const original = await f.manager.prepare(f.signal)
  const supplied = { ...immutableGeneration(original), sources: [] }
  // When / Then
  await expect(
    f.manager.refresh({ generation: supplied, signal: f.signal }),
  ).rejects.toThrow('does not belong')
})
