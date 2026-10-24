import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { mkdtempSync, realpathSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

/** Owned-tree cleanup contract, with verbatim successful owner stdout retained. */
export function retainedNativeFixture(helper: string, args: readonly string[]) {
  const root = realpathSync.native(
    mkdtempSync(join(tmpdir(), 'provenance-owned-native-')),
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
  // Preserve original identities and receipt before either assertion or deletion.
  process.stdout.write(result.stdout)
  process.stderr.write(result.stderr)
  console.info(
    'provenance-native runner ' + JSON.stringify({ status: result.status }),
  )
  assert(
    exited,
    `No owned tree exit for ${root}\n${result.stdout}${result.stderr}`,
  )
  try {
    assert.equal(result.status, 0, result.stdout + result.stderr)
  } finally {
    rmSync(root, { recursive: true })
  }
}
