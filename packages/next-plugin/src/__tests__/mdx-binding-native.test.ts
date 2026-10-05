import { join, resolve } from 'node:path'

import { expect, it } from 'bun:test'

it('preserves exact installed MDX compiler and fresh WASM class/CSS bytes with maps on and off', () => {
  // Given
  const workspace = resolve(import.meta.dir, '../../../..')
  const driver = join(import.meta.dir, 'mdx-binding-native.ts')
  // When
  const result = Bun.spawnSync([process.execPath, 'run', driver, workspace], {
    cwd: workspace,
    env: { ...process.env, NODE_ENV: 'test' },
    timeout: 30_000,
  })
  // Then
  expect(result.exitCode, result.stderr.toString()).toBe(0)
  const output: unknown = JSON.parse(result.stdout.toString())
  if (typeof output !== 'object' || output === null)
    throw new TypeError('invalid native fixture output')
  expect(output).toMatchObject({
    parity: true,
    compilerVersion: '3.1.1',
    identity: { packageVersion: '16.3.6' },
    rejectedPath: 'loaders[0].options.remarkPlugins[0]',
  })
})
