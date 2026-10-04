import { spawnSync } from 'node:child_process'
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { describe, expect, it } from 'bun:test'

describe('loader namespace cleanup across real coordinator suites', () => {
  it.each(
    [
      [
        'loader',
        'css-loader',
        'coordinator-dev',
        'coordinator-watch-lifecycle',
      ],
      [
        'css-loader',
        'loader',
        'coordinator-watch-lifecycle',
        'coordinator-dev',
      ],
      [
        'loader',
        'coordinator-dev',
        'css-loader',
        'coordinator-watch-lifecycle',
      ],
      [
        'css-loader',
        'coordinator-watch-lifecycle',
        'loader',
        'coordinator-dev',
      ],
    ].map((order) => ({ order })),
  )(
    'keeps real rebuilds isolated when suites run in order %j',
    ({ order }) => {
      // Given one subprocess with explicit suite scopes, not CLI file discovery.
      const dir = mkdtempSync(join(tmpdir(), 'devup-namespace-order-'))
      try {
        const fixture = join(dir, 'ordered.test.ts')
        writeFileSync(join(dir, 'bunfig.toml'), '[test]\ncoverage = false\n')
        writeFileSync(
          fixture,
          "import { describe } from 'bun:test'\n" +
            order
              .map(
                (file) =>
                  `describe(${JSON.stringify(file)}, () => { require(${JSON.stringify(join(import.meta.dir, `${file}.test.ts`))}) })`,
              )
              .join('\n'),
        )

        // When the existing loader and real rebuild/watcher tests share a realm.
        const result = spawnSync(process.execPath, ['test', fixture], {
          cwd: dir,
          encoding: 'utf8',
          timeout: 20000,
        })

        // Then every original assertion passes, including the factory guard.
        expect(result.status, result.stdout + result.stderr).toBe(0)
      } finally {
        rmSync(dir, { recursive: true, force: true })
      }
    },
    30000,
  )
})
