import { createRequire } from 'node:module'
import { join } from 'node:path'

import { describe, expect, it } from 'bun:test'

import { adapterFixture } from './adapter-fixture'

describe('caller final-config composition', () => {
  it.each(['replace', 'remove', 'keep'])(
    'uses the async caller return and restores its %s path',
    async (mode) => {
      const fixture = adapterFixture()
      try {
        const callerPath =
          fixture.caller(`let calls = 0; module.exports = { async modifyConfig(config, ctx) {
        calls++;
        if (!config.adapterPath.includes('caller-')) throw new Error('fake path');
        await new Promise(resolve => setImmediate(resolve));
        const output = { ...config, env: { caller: ctx.phase + ':' + ctx.projectDir, calls: String(calls), contextKeys: Object.keys(ctx).sort().join(',') }, assetPrefix: '/returned' };
        ${mode === 'replace' ? "output.adapterPath = './replacement.cjs';" : mode === 'remove' ? 'delete output.adapterPath;' : ''}
        return output;
      }};`)
        let calls = 0
        const installed = fixture.install(
          { adapterPath: callerPath, assetPrefix: '/input' },
          (effective) => {
            calls++
            expect(effective.assetPrefix).toBe('/returned')
            return { ...effective, trailingSlash: true }
          },
        )
        const result = await installed.adapter.modifyConfig(
          installed.config,
          fixture.context,
        )
        expect(result.adapterPath).toBe(
          mode === 'replace'
            ? './replacement.cjs'
            : mode === 'keep'
              ? callerPath
              : undefined,
        )
        expect(result.env?.caller).toBe(
          `${fixture.context.phase}:${fixture.context.projectDir}`,
        )
        expect(result.env?.calls).toBe('1')
        expect(result.env?.contextKeys).toBe('nextVersion,phase,projectDir')
        expect(calls).toBe(1)
      } finally {
        fixture.close()
      }
    },
  )

  it('retains a caller with only a build-complete hook', async () => {
    const fixture = adapterFixture()
    try {
      const callerPath = fixture.caller(
        'module.exports = { onBuildComplete() {} };',
      )
      const installed = fixture.install(
        { adapterPath: callerPath },
        (config) => config,
      )
      expect(
        (
          await installed.adapter.modifyConfig(
            installed.config,
            fixture.context,
          )
        ).adapterPath,
      ).toBe(callerPath)
      const caller: unknown = createRequire(
        join(fixture.session.projectDir, 'package.json'),
      )(callerPath)
      if (
        typeof caller !== 'object' ||
        caller === null ||
        !('onBuildComplete' in caller) ||
        typeof caller.onBuildComplete !== 'function'
      ) {
        throw new TypeError('fixture must expose onBuildComplete')
      }
      caller.onBuildComplete()
    } finally {
      fixture.close()
    }
  })

  it('resolves an ESM named adapter without a default export', async () => {
    const fixture = adapterFixture()
    try {
      const callerPath = fixture.caller(
        'export function modifyConfig(config) { return { ...config, trailingSlash: true } }',
        'mjs',
      )
      const installed = fixture.install(
        { adapterPath: callerPath },
        (config) => config,
      )
      expect(
        (
          await installed.adapter.modifyConfig(
            installed.config,
            fixture.context,
          )
        ).trailingSlash,
      ).toBe(true)
    } finally {
      fixture.close()
    }
  })

  it('preserves a callable CommonJS adapter with a modifyConfig method', async () => {
    const fixture = adapterFixture()
    try {
      const callerPath = fixture.caller(
        'function adapter() {} adapter.modifyConfig = c => ({ ...c, trailingSlash: true }); module.exports = adapter;',
      )
      const installed = fixture.install(
        { adapterPath: callerPath },
        (config) => config,
      )
      expect(
        (
          await installed.adapter.modifyConfig(
            installed.config,
            fixture.context,
          )
        ).trailingSlash,
      ).toBe(true)
      const caller: unknown = createRequire(
        join(fixture.session.projectDir, 'package.json'),
      )(callerPath)
      if (typeof caller !== 'function')
        throw new TypeError('fixture must be callable')
      caller()
    } finally {
      fixture.close()
    }
  })

  it('preserves an environment deployment adapter when no caller path is supplied', async () => {
    const fixture = adapterFixture()
    const previous = process.env.NEXT_ADAPTER_PATH
    try {
      const callerPath = fixture.caller(
        'module.exports = { modifyConfig(c) { return { ...c, assetPrefix: "/platform" } } };',
      )
      process.env.NEXT_ADAPTER_PATH = callerPath
      const installed = fixture.install({}, (config) => config)
      const result = await installed.adapter.modifyConfig(
        installed.config,
        fixture.context,
      )
      expect(result.adapterPath).toBe(callerPath)
      expect(result.assetPrefix).toBe('/platform')
    } finally {
      if (previous === undefined) delete process.env.NEXT_ADAPTER_PATH
      else process.env.NEXT_ADAPTER_PATH = previous
      fixture.close()
    }
  })

  it('preserves the legacy caller path without letting migration bypass the temporary adapter', async () => {
    const fixture = adapterFixture()
    try {
      const callerPath = fixture.caller(
        'module.exports = { modifyConfig(c) { return { ...c, trailingSlash: true } } };',
      )
      const original = {
        adapterPath: '/unused.cjs',
        experimental: { cpus: 2, adapterPath: callerPath },
      }
      const installed = fixture.install(original, (config) => config)
      expect(installed.config.experimental).toEqual({ cpus: 2 })
      const result = await installed.adapter.modifyConfig(
        installed.config,
        fixture.context,
      )
      expect(result.adapterPath).toBe(callerPath)
      expect(result.trailingSlash).toBe(true)
      expect(original.experimental.adapterPath).toBe(callerPath)
    } finally {
      fixture.close()
    }
  })
})
