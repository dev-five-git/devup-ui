import { spawnSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'

import { describe, expect, it } from 'bun:test'

describe('export', () => {
  it('should export DevupUIWebpackPlugin', async () => {
    const index = await import('../index')
    expect({ ...index }).toEqual({
      DevupUIWebpackPlugin: expect.any(Function),
      createWebpackGeneration: expect.any(Function),
      registerWebpackReturnedConfig: expect.any(Function),
      readWebpackProductionManifest: expect.any(Function),
      sealWebpackProductionManifest: expect.any(Function),
      WebpackProductionManifestError: expect.any(Function),
    })
  })
})

it('shares actual owner authority when advertised built CJS and ESM entries interleave', () => {
  // Given
  const cwd = fileURLToPath(new URL('../..', import.meta.url))
  const script = `
    import assert from 'node:assert/strict'
    import { createRequire } from 'node:module'
    const require = createRequire(process.cwd() + '/package.json')
    const cjs = require('@devup-ui/webpack-plugin')
    const esm = await import('@devup-ui/webpack-plugin')
    assert.equal(require.resolve('@devup-ui/webpack-plugin').endsWith('.cjs'), true)
    assert.equal(import.meta.resolve('@devup-ui/webpack-plugin').endsWith('.mjs'), true)
    assert.notEqual(cjs.registerWebpackReturnedConfig, esm.registerWebpackReturnedConfig)
    for (const [creator, writer] of [[cjs, esm], [esm, cjs]]) {
      const owner = creator.createWebpackGeneration()
      const config = { context: '/built-actual', name: 'native', entry: { main: './first.ts' } }
      writer.registerWebpackReturnedConfig(owner, 'client', config)
      assert.equal(creator.readWebpackProductionManifest(owner).configs.client[0], config)
      const sealed = creator.sealWebpackProductionManifest(owner)
      assert.equal(writer.readWebpackProductionManifest(owner), sealed)
      assert.throws(() => writer.registerWebpackReturnedConfig(owner, 'client', config),
        { code: 'sealed', operation: 'register', coordinates: [{ role: 'client', context: '/built-actual', name: 'native' }] })
      config.entry.main = './changed.ts'
      assert.equal(sealed.configs.client[0].entry.main, './changed.ts')
      owner.acquire(true)()
      assert.equal(owner.disposed, true)
      for (const api of [creator, writer]) {
        assert.throws(() => api.readWebpackProductionManifest(owner), { code: 'disposed', operation: 'read' })
        assert.throws(() => api.sealWebpackProductionManifest(owner), { code: 'disposed', operation: 'seal' })
        assert.throws(() => api.registerWebpackReturnedConfig(owner, 'edge', {}), { code: 'disposed', operation: 'register' })
      }
    }
    console.log('mixed-entry-authority: passed')
  `
  // When
  const result = spawnSync('node', ['--input-type=module', '-e', script], {
    cwd,
    encoding: 'utf8',
  })
  // Then
  expect(result.error).toBeUndefined()
  expect({ status: result.status, stderr: result.stderr }).toEqual({
    status: 0,
    stderr: '',
  })
  expect(result.stdout.trim()).toBe('mixed-entry-authority: passed')
})
