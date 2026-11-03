import { spawnSync } from 'node:child_process'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

it('drains native descendants when the owned command exceeds its deadline', () => {
  // Given: a real parent and grandchild that both ignore graceful shutdown.
  const owner = join(import.meta.dirname, 'native-webpack-owned.cjs')
  const child = join(import.meta.dirname, 'webpack-lifecycle-child.cjs')
  // When: the foreground owner reaches its bounded command deadline.
  const result = spawnSync('node', [owner, '3000', child], {
    encoding: 'utf8',
  })
  // Then: timeout remains failure and every observed native member is gone.
  console.info(result.stdout)
  expect(result.error).toBeUndefined()
  expect(result.status).toBe(124)
  expect(result.stdout).toMatch(/"active":\[\]/)
  const pids = [...result.stdout.matchAll(/FAULT-PID (\d+)/g)]
  expect(pids.length).toBe(2)
  for (const match of pids) {
    expect(() => process.kill(Number(match[1]), 0)).toThrow()
  }
}, 20000)

for (const signal of ['SIGTERM', 'SIGINT']) {
  for (const pipes of ['open', 'closed']) {
    it(`drains native descendants when the actual owner receives ${signal} with ${pipes} output`, () => {
      // Given: a probe that waits for two live fault descendants.
      const probe = join(import.meta.dirname, 'native-webpack-cancel.cjs')
      // When: the probe cancels the actual command owner, not its child deadline.
      const result = spawnSync('node', [probe, signal, pipes], {
        encoding: 'utf8',
      })
      // Then: the probe returns only after non-success cancellation and full drain.
      console.info(result.stdout)
      expect({ status: result.status, stderr: result.stderr }).toEqual({
        status: 0,
        stderr: '',
      })
      expect(result.error).toBeUndefined()
      expect(result.stdout).toMatch(/"event":"owner_cancel"/)
    }, 25000)
  }
}
