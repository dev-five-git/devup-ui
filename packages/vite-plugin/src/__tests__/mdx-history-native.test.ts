import { spawnSync } from 'node:child_process'
import { resolve } from 'node:path'

import { expect, it } from 'bun:test'

it('preserves learned history when main fails independently of sibling validation', () => {
  // Given an isolated real Vite/MDX history scenario.
  const fixture = resolve(import.meta.dir, 'mdx-history-native.mjs')
  // When native builds fail proof and then repair with fresh participation.
  const result = spawnSync(
    'node',
    [
      resolve(
        import.meta.dir,
        '../../../next-plugin/src/__tests__/native-webpack-owned.cjs',
      ),
      '120000',
      fixture,
    ],
    { encoding: 'utf8', timeout: 150000 },
  )
  // Then the actual history and placeholder assertions pass in both modes.
  console.info(result.stdout)
  expect(result.status, result.stdout + result.stderr).toBe(0)
}, 180000)
