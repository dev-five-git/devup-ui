import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { mkdtempSync, realpathSync, rmSync, symlinkSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { it } from 'bun:test'

const owner = resolve(
  import.meta.dir,
  '../../../next-plugin/src/__tests__/native-webpack-owned.cjs',
)

it.each(['text/css', '*/*'])(
  'serves generated CSS with the right representation and revalidatable caching when Accept is %s',
  async (accept) => {
    // Given: a unique bundle of the SOURCE plugin, not a possibly stale dist.
    const temp =
      process.platform === 'win32' && process.env.LOCALAPPDATA
        ? join(process.env.LOCALAPPDATA, 'Temp')
        : tmpdir()
    const root = realpathSync(mkdtempSync(join(temp, 'devup-css-http-')))
    try {
      symlinkSync(
        resolve(import.meta.dir, '../../node_modules'),
        join(root, 'node_modules'),
        process.platform === 'win32' ? 'junction' : 'dir',
      )
      const bundle = await Bun.build({
        entrypoints: [resolve(import.meta.dir, 'css-http-native.ts')],
        outdir: root,
        naming: 'probe.mjs',
        target: 'node',
        format: 'esm',
        packages: 'external',
      })
      assert(bundle.success, bundle.logs.map(String).join('\n'))

      // When: native Node owns Vite and HTTP, avoiding Bun's Windows fs identity.
      const result = spawnSync(
        'node',
        [owner, '45000', join(root, 'probe.mjs'), accept],
        { encoding: 'utf8', timeout: 60000, windowsHide: true },
      )

      // Then: all real HTTP assertions passed and the owned process tree drained.
      assert.equal(result.error, undefined)
      assert.equal(result.status, 0, result.stdout + result.stderr)
    } finally {
      rmSync(root, { recursive: true, force: true })
    }
  },
  90000,
)
