import { spawnSync } from 'node:child_process'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

it('selects blue in a genuine same-PID native process after exports-only setup reconstruction', () => {
  // Given the checked-in persistent native driver and built public package.
  const driver = join(import.meta.dir, 'tsconfig-native-freshness.mjs')
  // When the foreground child inherits the caller's process guard.
  const result = spawnSync('node', [driver], {
    encoding: 'utf8',
    timeout: 20000,
  })
  // Then the driver proves fresh selection and zero repeated inheritance reads.
  expect({ status: result.status, stderr: result.stderr }).toEqual({
    status: 0,
    stderr: '',
  })
  expect(JSON.parse(result.stdout)).toMatchObject({
    runtime: 'node',
    before: 'red',
    after: 'blue',
  })
})
