import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { resolve } from 'node:path'

import { it } from 'bun:test'

for (const [probe, mode] of [
  ['admission', 'config'],
  ['admission', 'tailwind'],
  ['admission', 'tailwind-import'],
  ['build-lease', 'overlap'],
  ['build-lease', 'solo'],
  ['build-lease', 'pending'],
]) {
  it(`preserves real prerequisite behavior when ${probe} ${mode}`, () => {
    // Given / When
    const result = spawnSync(
      'node',
      [
        resolve(
          import.meta.dir,
          '../../../next-plugin/src/__tests__/native-webpack-owned.cjs',
        ),
        '120000',
        resolve(import.meta.dir, `serve-coherence-${probe}-native.cjs`),
        mode,
      ],
      { encoding: 'utf8', timeout: 150000 },
    )
    // Then
    assert.equal(result.status, 0, result.stdout + result.stderr)
    assert.match(
      result.stdout,
      /serve-coherence-final-complete/,
      result.stdout + result.stderr,
    )
  }, 180000)
}
