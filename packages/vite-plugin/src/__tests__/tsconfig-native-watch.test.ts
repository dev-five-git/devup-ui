import { spawnSync } from 'node:child_process'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

it('reextracts blue CSS through public invalidation in persistent genuine native dev', () => {
  // Given the checked-in manifest-only native fixture and built public plugin.
  const driver = join(import.meta.dir, 'tsconfig-native-watch.mjs')
  // When native Node owns the entire awaited server lifecycle without a restart.
  const result = spawnSync('node', [driver], {
    encoding: 'utf8',
    timeout: 25000,
  })
  // Then its actual selected class has a blue declaration, not merely a watcher event.
  expect({ status: result.status, stderr: result.stderr }).toEqual({
    status: 0,
    stderr: '',
  })
  expect(result.stdout).toContain('background:blue')
}, 30000)
