import { spawnSync } from 'node:child_process'
import { resolve } from 'node:path'

import { expect, it } from 'bun:test'

it.each([false, true])(
  'qualifies operative raw native controls with singleCss=%s',
  (singleCss) => {
    // Given cold ordinary/inert controls and independently compiled unrelated MDX.
    // When native fallback aliasing or externalization actually changes the edge.
    const result = spawnSync(
      'node',
      [
        resolve(
          import.meta.dir,
          '../../../next-plugin/src/__tests__/native-webpack-owned.cjs',
        ),
        '120000',
        resolve(import.meta.dir, 'mdx-raw-controls-native.mjs'),
        String(singleCss),
      ],
      { encoding: 'utf8', timeout: 150000 },
    )
    // Then positives retain exact identity and operative negatives refuse located.
    console.info(result.stdout)
    expect(result.status, result.stdout + result.stderr).toBe(0)
  },
  180000,
)
