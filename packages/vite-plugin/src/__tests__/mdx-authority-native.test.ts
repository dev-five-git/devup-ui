import { spawnSync } from 'node:child_process'
import { mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { expect, it } from 'bun:test'

it.each([false, true])(
  'refuses unproven outer MDX decisions with singleCss=%s',
  (singleCss) => {
    // Given real Vite resolver hooks and the configured MDX compiler.
    const root = mkdtempSync(join(tmpdir(), 'devup-mdx-authority-'))
    try {
      // When aliases and earlier hooks choose an unproven default target.
      const result = spawnSync(
        'node',
        [
          resolve(
            import.meta.dir,
            '../../../next-plugin/src/__tests__/native-webpack-owned.cjs',
          ),
          '120000',
          resolve(import.meta.dir, 'mdx-authority-native.mjs'),
          root,
          String(singleCss),
        ],
        { encoding: 'utf8', timeout: 150000 },
      )
      console.info(result.stdout)
      // Then every successful output is placeholder-free; uncertainty fails located.
      expect(result.stdout + result.stderr).not.toContain('wall-clock-limit')
      expect(result.status, result.stdout + result.stderr).toBe(0)
    } finally {
      rmSync(root, { recursive: true, force: true })
    }
  },
  180000,
)
