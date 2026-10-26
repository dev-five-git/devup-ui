import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { resolve } from 'node:path'

import { it } from 'bun:test'

for (const [probe, stage] of [
  ['refresh', 'load'],
  ['refresh', 'write'],
  ['lifetime', 'before-refresh'],
  ['lifetime', 'load'],
  ['lifetime', 'buildStart'],
  ['lifetime', 'transform'],
]) {
  it(`keeps real async coherence when ${probe} pauses at ${stage}`, () => {
    // Given / When
    const result = spawnSync(
      'node',
      [
        resolve(
          import.meta.dir,
          '../../../next-plugin/src/__tests__/native-webpack-owned.cjs',
        ),
        '120000',
        resolve(import.meta.dir, `serve-coherence-async-${probe}.cjs`),
        stage,
      ],
      { encoding: 'utf8', timeout: 150000 },
    )
    // Then
    assert.equal(result.status, 0, result.stdout + result.stderr)
    assert.match(
      result.stdout,
      /serve-coherence-async-complete/,
      result.stdout + result.stderr,
    )
  }, 180000)
}
