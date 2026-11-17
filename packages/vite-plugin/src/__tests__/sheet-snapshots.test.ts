import { spawnSync } from 'node:child_process'
import { realpathSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterAll, expect, it } from 'bun:test'

// Vite cannot resolve modules under a Windows 8.3 short path
const root = join(realpathSync.native(tmpdir()), 'devup-sheet-snapshots')
afterAll(() => rmSync(root, { recursive: true, force: true }))

it('writes every sheet to its own file when two are equal as they load', () => {
  // When
  const result = spawnSync(
    process.execPath,
    [join(import.meta.dir, 'sheet-snapshots-build.ts'), root],
    { encoding: 'utf8' },
  )
  // Then
  expect(result.stderr).toBe('')
  expect(JSON.parse(result.stdout)).toEqual({
    client: {
      'devup-ui-0.css': '.a-a{color:red}',
      'devup-ui-2.css': '',
      'devup-ui.css': '.a{color:blue}',
    },
    server: {
      'devup-ui-1.css': '.b-a{color:green}',
      'devup-ui-2.css': '',
      'devup-ui.css': '.a{color:blue}',
    },
  })
}, 120_000)
