import { join } from 'node:path'

import { expect, it } from 'bun:test'

it('serves the new selected CSS rule when only linked workspace exports change', () => {
  // Given the real Rsbuild dev-server fixture and two existing palette entries.
  const runner = join(import.meta.dir, 'resolution-watch.mjs')
  // When only exports is edited and public compilation callbacks complete.
  const result = Bun.spawnSync(['node', runner], {
    stdout: 'pipe',
    stderr: 'pipe',
  })
  // Then HTTP-served CSS selects blue and the separate cold build has no red.
  expect({
    exitCode: result.exitCode,
    error: result.stderr.toString(),
  }).toMatchObject({ exitCode: 0 })
  expect(result.stdout.toString()).toContain('"after"')
}, 45000)
