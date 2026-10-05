import { createRequire } from 'node:module'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { sourceFixture, styledMdx } from './mdx-source-fixture'

function observations(root: string): readonly unknown[] {
  const raw: unknown = createRequire(join(root, 'package.json'))(
    join(root, 'reporting-raw.cjs'),
  )
  if (typeof raw !== 'function' || !('seen' in raw) || !Array.isArray(raw.seen))
    throw new TypeError('Missing native options observations')
  return raw.seen
}

it('reuses one actual native options instance across stable pipelines in a preparation run', async () => {
  // Given
  const f = sourceFixture({
    'app/a/page.mdx': styledMdx,
    'app/b/page.mdx': styledMdx,
  })
  // When
  await f.manager.prepare(f.signal)
  const seen = observations(f.root)
  // Then
  expect(seen).toHaveLength(2)
  expect(seen[0]).toBe(seen[1])
})

it('creates a new actual options instance for a changed preparation run', async () => {
  // Given
  const f = sourceFixture({ 'app/page.mdx': styledMdx })
  const first = await f.manager.prepare(f.signal)
  const original = observations(f.root)[0]
  f.write('app/page.mdx', styledMdx.replace('red', 'green'))
  // When
  await f.manager.refresh({ generation: first, signal: f.signal })
  // Then
  const seen = observations(f.root)
  expect(seen).toHaveLength(2)
  expect(seen[1]).not.toBe(original)
})
