import assert from 'node:assert/strict'
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { it } from 'bun:test'

for (const cssMode of ['single', 'per-file']) {
  for (const order of ['AB', 'BA']) {
    it(`delivers exact current ${cssMode} watch CSS with late layers in ${order} order`, () => {
      // Given: the original native fixture and a genuine late .css.ts callback.
      const archive = process.env.DEVUP_WATCH_ARCHIVE
      const fixture = mkdtempSync(
        join(archive ?? tmpdir(), `watch-${cssMode}-${order}-`),
      )
      try {
        // When: native webpack delivers the original initial and revised callbacks.
        const child = Bun.spawnSync(
          [
            'node',
            '--require',
            join(import.meta.dir, 'watch-generation-observer.cjs'),
            join(import.meta.dir, 'reservation-build.cjs'),
            fixture,
            order,
            'watch',
            cssMode,
          ],
          {
            stdout: 'pipe',
            stderr: 'pipe',
            timeout: 45000,
            env: {
              ...process.env,
              DEVUP_WATCH_RECEIPT: join(fixture, 'receipt.jsonl'),
            },
          },
        )
        if (archive) {
          writeFileSync(`${fixture}.stdout.txt`, child.stdout)
          writeFileSync(`${fixture}.stderr.txt`, child.stderr)
        }
        // Then: every callback's real CSS modules equal the complete current engine.
        assert.equal(
          child.exitCode,
          0,
          child.stdout.toString() + child.stderr.toString(),
        )
      } finally {
        if (!archive) rmSync(fixture, { recursive: true, force: true })
      }
    }, 50000)
  }
}
