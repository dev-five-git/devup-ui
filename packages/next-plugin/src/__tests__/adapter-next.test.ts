import { spawnSync } from 'node:child_process'
import { mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { expect, it } from 'bun:test'

it('captures installed Next finalization for both MDX wrapper orders and bundlers across isolated modules', async () => {
  // Given: separate bundles reproduce Next config-module isolation and a shipped CJS entry.
  const root = resolve(import.meta.dir, '../../../..')
  const artifacts = mkdtempSync(join(tmpdir(), 'devup-adapter-next-'))
  try {
    for (const [entry, outfile] of [
      ['final-config-adapter.ts', 'installer.cjs'],
      ['final-config-adapter.ts', 'installer-reload.cjs'],
      ['final-config-adapter-entry.ts', 'entry.cjs'],
    ] as const) {
      const build = await Bun.build({
        entrypoints: [resolve(import.meta.dir, '..', entry)],
        outdir: artifacts,
        naming: outfile,
        format: 'cjs',
        target: 'node',
        packages: 'external',
      })
      expect(build.success).toBe(true)
    }
    // When: actual Next loads async config functions, applies the adapter, then finalizes.
    const capture = spawnSync(
      'node',
      [join(import.meta.dir, 'adapter-next-capture.cjs'), root, artifacts],
      { encoding: 'utf8', timeout: 45000 },
    )
    // Then: the native source/config assertions and 12-case matrix all succeeded.
    expect(capture.stderr).toBe('')
    expect(capture.status).toBe(0)
    const result: unknown = JSON.parse(capture.stdout)
    if (
      typeof result !== 'object' ||
      result === null ||
      !('captures' in result) ||
      !Array.isArray(result.captures)
    ) {
      throw new TypeError('capture must return the tested matrix')
    }
    expect(result.captures).toHaveLength(12)
  } finally {
    rmSync(artifacts, { recursive: true, force: true })
  }
}, 60000)
