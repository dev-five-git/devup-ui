import { spawnSync } from 'node:child_process'
import { resolve } from 'node:path'

import { expect, it } from 'bun:test'

it.each([false, true])(
  'proves only current direct final-demand erasure with singleCss=%s',
  (singleCss) => {
    // Given actual native resolution and MDX compilation.
    // When final transforms strip roots/targets or preserve hostile demand.
    const result = spawnSync(
      'node',
      [
        resolve(
          import.meta.dir,
          '../../../next-plugin/src/__tests__/native-webpack-owned.cjs',
        ),
        '120000',
        resolve(import.meta.dir, 'mdx-erasure-native.mjs'),
        String(singleCss),
      ],
      { encoding: 'utf8', timeout: 150000 },
    )
    // Then positives remain placeholder-free and hostile builds refuse located.
    console.info(result.stdout)
    expect(result.stdout + result.stderr).not.toContain('wall-clock-limit')
    expect(result.status, result.stdout + result.stderr).toBe(0)
  },
  180000,
)
