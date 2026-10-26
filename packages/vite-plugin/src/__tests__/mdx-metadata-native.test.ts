import { spawnSync } from 'node:child_process'
import { resolve } from 'node:path'

import { expect, it } from 'bun:test'

it.each([false, true])(
  'retains owned default obligations when public metadata changes with singleCss=%s',
  (singleCss) => {
    // Given real native builds with explicit transport mutation controls.
    // When current emitted obligations are deleted or replaced through moduleParsed.
    const result = spawnSync(
      'node',
      [
        resolve(
          import.meta.dir,
          '../../../next-plugin/src/__tests__/native-webpack-owned.cjs',
        ),
        '120000',
        resolve(import.meta.dir, 'mdx-metadata-native.mjs'),
        String(singleCss),
      ],
      { encoding: 'utf8', timeout: 150000 },
    )
    // Then all variants retain the original located build refusal.
    console.info(result.stdout)
    expect(result.status, result.stdout + result.stderr).toBe(0)
  },
  180000,
)
