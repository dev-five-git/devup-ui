import { readFileSync } from 'node:fs'
import { join } from 'node:path'

import { describe, expect, it } from 'bun:test'
import type { NextConfig } from 'next'

import {
  FinalConfigAdapterError,
  installFinalConfigAdapter,
} from '../final-config-adapter'
import { createFinalConfigAdapter } from '../final-config-adapter-entry'
import { adapterFixture } from './adapter-fixture'

describe('final-config registration lifecycle', () => {
  it('awaits a controlled slow finalizer without changing prior hooks or props', async () => {
    // Given: a real pending finalizer and existing config hooks.
    const fixture = adapterFixture()
    const entered = Promise.withResolvers<void>()
    const gate = Promise.withResolvers<void>()
    const webpack = () => ({})
    const hook = async () => {}
    const config: NextConfig = {
      webpack,
      compiler: { runAfterProductionCompile: hook },
      env: { retained: 'yes' },
    }
    try {
      const installed = fixture.install(
        config,
        async (effective, context, signal) => {
          expect(context).toBe(fixture.context)
          expect(signal.aborted).toBe(false)
          entered.resolve()
          await gate.promise
          return { ...effective, pageExtensions: ['mdx', 'tsx'] }
        },
      )
      // When: start the hook; prove it remains pending at the explicit gate.
      let settled = false
      const pending = installed.adapter.modifyConfig(
        installed.config,
        fixture.context,
      )
      void pending.then(() => {
        settled = true
      })
      await entered.promise
      expect(settled).toBe(false)
      gate.resolve()
      const result = await pending
      // Then: original config surface survives, with no temporary deployment adapter.
      expect(result).toEqual({ ...config, pageExtensions: ['mdx', 'tsx'] })
      expect(result.webpack).toBe(webpack)
      expect(result.compiler?.runAfterProductionCompile).toBe(hook)
      expect(config.adapterPath).toBeUndefined()
      expect(
        readFileSync(installed.config.adapterPath ?? '', 'utf8'),
      ).not.toContain('runAfterProductionCompile')
    } finally {
      fixture.close()
    }
  })

  it('rejects released or foreign-project tokens', async () => {
    const fixture = adapterFixture()
    try {
      const installed = fixture.install({}, (config) => config)
      await expect(
        installed.adapter.modifyConfig(installed.config, {
          ...fixture.context,
          projectDir: '/',
        }),
      ).rejects.toMatchObject({ stage: 'project ownership' })
      installed.release()
      installed.release()
      await expect(
        createFinalConfigAdapter(fixture.session.token).modifyConfig(
          {},
          fixture.context,
        ),
      ).rejects.toMatchObject({ stage: 'registry lookup' })
      const complete = {
        ...installed.config,
        configFile: fixture.session.configFile,
      }
      await expect(
        installed.adapter.modifyConfig(complete, fixture.context),
      ).rejects.toMatchObject({
        stage: 'registry lookup',
        location: fixture.session.configFile,
      })
    } finally {
      fixture.close()
    }
  })

  it('rejects an invalid legacy path before generating an adapter', () => {
    const fixture = adapterFixture()
    try {
      const original = { experimental: { cpus: 2, adapterPath: 1 } }
      expect(() => fixture.install(original, (config) => config)).toThrow(
        FinalConfigAdapterError,
      )
    } finally {
      fixture.close()
    }
  })

  it('rejects duplicate ownership and invalid budgets without overwriting the owner', () => {
    const fixture = adapterFixture()
    try {
      fixture.install({}, (config) => config)
      expect(() => fixture.install({}, (config) => config)).toThrow(
        FinalConfigAdapterError,
      )
      expect(() => fixture.install({}, (config) => config, 45001)).toThrow(
        FinalConfigAdapterError,
      )
    } finally {
      fixture.close()
    }
  })

  it('locates a temporary adapter write failure', () => {
    const fixture = adapterFixture()
    try {
      expect(() =>
        installFinalConfigAdapter(
          {},
          {
            ...fixture.session,
            sessionDir: fixture.session.projectDir + '/absent',
          },
          {
            entryPath: join(fixture.session.projectDir, 'shipped-entry.cjs'),
            finalize: (config) => config,
          },
        ),
      ).toThrow(FinalConfigAdapterError)
    } finally {
      fixture.close()
    }
  })
})
