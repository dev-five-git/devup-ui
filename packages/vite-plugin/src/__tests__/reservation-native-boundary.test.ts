import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { resolve } from 'node:path'

import { it } from 'bun:test'

it('preserves actual-input native manifest and supplier events inside the named watch boundary', () => {
  // Given / When
  const result = spawnSync(
    'node',
    [
      resolve(
        import.meta.dir,
        '../../../next-plugin/src/__tests__/native-webpack-owned.cjs',
      ),
      '120000',
      resolve(import.meta.dir, 'reservation-native-boundary.cjs'),
    ],
    { encoding: 'utf8', timeout: 150000 },
  )
  // Then
  assert.equal(result.status, 0, result.stdout + result.stderr)
}, 180000)
