import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { mkdtempSync, realpathSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

export function ownedNativeFixture(
  helper: string,
  args: readonly string[],
  inspect: (root: string) => void,
) {
  const root = realpathSync(
    mkdtempSync(join(tmpdir(), 'serve-coherence-owned-native-')),
  )
  const result = spawnSync(
    'node',
    [
      resolve(
        import.meta.dir,
        '../../../next-plugin/src/__tests__/native-webpack-owned.cjs',
      ),
      '120000',
      helper,
      ...args,
      root,
    ],
    { encoding: 'utf8', timeout: 150000 },
  )
  const exited = result.stdout.split(/\r?\n/).some((line) => {
    const prefix = 'NATIVE-LIFECYCLE '
    if (!line.startsWith(prefix)) return false
    const value: unknown = JSON.parse(line.slice(prefix.length))
    return (
      typeof value === 'object' &&
      value !== null &&
      'event' in value &&
      value.event === 'tree_exit' &&
      'active' in value &&
      Array.isArray(value.active) &&
      value.active.length === 0
    )
  })
  // An unconfirmed owner exit is fatal: do not race its unknown live handles.
  assert(
    exited,
    `No owned tree exit for ${root}\n${result.stdout}${result.stderr}`,
  )
  try {
    assert.equal(result.status, 0, result.stdout + result.stderr)
    inspect(root)
  } finally {
    rmSync(root, { recursive: true })
  }
}
