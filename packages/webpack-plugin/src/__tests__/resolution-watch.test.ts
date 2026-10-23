import { join } from 'node:path'

import { expect, it } from 'bun:test'

it.each(['canonical', 'preserve'])(
  'refreshes actual watch CSS when only linked package exports change with %s symlinks',
  (mode) => {
    // Given the real compiler fixture with two preexisting palette entries.
    const runner = join(import.meta.dir, 'resolution-watch.mjs')
    // When the runner edits only exports and awaits actual compiler callbacks.
    const result = Bun.spawnSync(['node', runner, mode], {
      stdout: 'pipe',
      stderr: 'pipe',
    })
    // Then emitted current-class CSS is blue and a cold build has no red.
    expect({
      exitCode: result.exitCode,
      error: result.stderr.toString(),
    }).toMatchObject({ exitCode: 0 })
    expect(result.stdout.toString()).toContain('"after"')
  },
  45000,
)
