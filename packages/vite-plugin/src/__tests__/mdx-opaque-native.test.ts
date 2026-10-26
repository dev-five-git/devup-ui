import { spawnSync } from 'node:child_process'
import { mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { expect, it } from 'bun:test'

it.each([false, true])(
  'enforces no-staging MDX contracts with singleCss=%s',
  (single) => {
    // Given the real configured compiler and a normal built plugin in an owned child.
    const root = mkdtempSync(join(tmpdir(), 'devup-opaque-mdx-'))
    try {
      // When native compilation exercises default rendering, forbidden reads and repair.
      const result = spawnSync(
        'node',
        [
          resolve(
            import.meta.dir,
            '../../../next-plugin/src/__tests__/native-webpack-owned.cjs',
          ),
          '120000',
          resolve(import.meta.dir, 'mdx-opaque-native.mjs'),
          root,
          String(single),
        ],
        { encoding: 'utf8', timeout: 150000 },
      )
      console.info(result.stdout)
      // Then every successful output is runtime-placeholder free and every refusal located.
      expect(result.stdout + result.stderr).not.toContain('wall-clock-limit')
      expect(result.status, result.stdout + result.stderr).toBe(0)
    } finally {
      rmSync(root, { recursive: true, force: true })
    }
  },
  180000,
)
