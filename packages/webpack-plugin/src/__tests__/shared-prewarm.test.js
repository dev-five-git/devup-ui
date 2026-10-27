import { spawnSync } from 'node:child_process'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

it('preserves static and dynamic identities when excluded client entries join a shared prewarm generation', () => {
  // Given an isolated real WASM engine, not the unit-suite mocks.
  // When server fallback and physical client entries configure the same owner.
  const result = spawnSync(
    process.execPath,
    [join(import.meta.dir, 'shared-prewarm.mjs')],
    { encoding: 'utf8' },
  )
  // Then sibling extraction neither changes selected identities nor allocates duplicates.
  if (result.status !== 0) process.stderr.write(result.stdout + result.stderr)
  expect(result.error).toBeUndefined()
  expect(result.status, result.stderr).toBe(0)
})
