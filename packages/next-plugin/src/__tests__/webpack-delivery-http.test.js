import { spawnSync } from 'node:child_process'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

it('rejects broken HTTP delivery boundaries in the native Node surface', () => {
  // Given native Node fetch, isolated from the suite DOM preload.
  // When real HTTP documents or their linked CSS violate delivery contracts.
  const result = spawnSync(
    'node',
    [join(import.meta.dir, 'webpack-delivery-negative.cjs')],
    { encoding: 'utf8' },
  )
  // Then each rejection must identify the broken boundary, not a later error.
  console.info(result.stdout + result.stderr)
  expect(result.error).toBeUndefined()
  expect(result.status).toBe(0)
})
