import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { expect, it } from 'bun:test'
import type { Compiler, Configuration } from 'webpack'

import { DevupUIWebpackPlugin } from '../plugin'
import { mdxLanding } from './mdx-build-fixture'

const bundled: { webpack(config: Configuration): Compiler } = mdxLanding(
  'next/dist/compiled/webpack/webpack',
)

it('aborts native compiler setup when the real resolver reads a malformed tsconfig', () => {
  const root = mkdtempSync(join(tmpdir(), 'devup-root-webpack-error-'))
  writeFileSync(join(root, 'tsconfig.json'), '{ malformed')
  try {
    expect(() =>
      bundled.webpack({
        context: root,
        mode: 'production',
        entry: './entry.ts',
        plugins: [new DevupUIWebpackPlugin({ watch: false })],
      }),
    ).toThrow('resolver setup failed')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})
