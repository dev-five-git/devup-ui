import { spawnSync } from 'node:child_process'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

it('reconstructs complete production sheets across compiler resets without leaking to another build', () => {
  // Given an isolated real engine (other unit tests mock WASM exports).
  // When applying real compilers, awaiting their own preparation tap, and closing them.
  const result = spawnSync(
    process.execPath,
    [join(import.meta.dir, 'production-prewarm.mjs')],
    {
      encoding: 'utf8',
    },
  )
  // Then all adapter selection, reconstruction and isolation assertions pass.
  expect(result.error).toBeUndefined()
  expect(result.stdout + result.stderr).toContain(
    'production prewarm proof passed',
  )
  expect(result.status).toBe(0)
})

it('awaits every outstanding shutdown before removing fixtures and preserves proof plus cleanup errors', () => {
  // Given asynchronous failing shutdowns and a proof assertion failure.
  // When the real-engine harness unwinds.
  const result = spawnSync(
    process.execPath,
    [join(import.meta.dir, 'production-prewarm.mjs')],
    {
      encoding: 'utf8',
      env: { ...process.env, DEVUP_PREWARM_CLEANUP_PROOF: '1' },
    },
  )
  // Then both callbacks finish with the fixture alive, and all three errors survive.
  expect(result.error).toBeUndefined()
  expect(result.status).toBe(1)
  expect(result.stdout.trim().split(/\r?\n/)).toEqual([
    'shutdown:first',
    'shutdown:second',
    'fixture-removed',
  ])
  expect(result.stderr).toContain('initialization-proof-failure')
  expect(result.stderr).toContain('cleanup:first')
  expect(result.stderr).toContain('cleanup:second')
})
