import { expect, it } from 'bun:test'

import { FinalConfigAdapterError } from '../final-config-adapter'
import { adapterFixture } from './adapter-fixture'

it.each([
  'throw new Error("caller failure")',
  'throw "non-error caller failure"',
  'return null',
  'return []',
  'return { adapterPath: 42 }',
])(
  'locates invalid/failed caller hook %# without running the finalizer',
  async (body) => {
    const fixture = adapterFixture()
    try {
      const callerPath = fixture.caller(
        `module.exports = { modifyConfig() { ${body} } };`,
      )
      let finalized = false
      const installed = fixture.install(
        { adapterPath: callerPath },
        (config) => {
          finalized = true
          return config
        },
      )
      const failure = await installed.adapter
        .modifyConfig(installed.config, fixture.context)
        .catch((cause: unknown) => cause)
      expect(failure).toBeInstanceOf(FinalConfigAdapterError)
      expect(failure).toMatchObject({
        context: fixture.context,
        location: fixture.session.configFile,
        stage: 'caller modifyConfig',
      })
      expect(finalized).toBe(false)
    } finally {
      fixture.close()
    }
  },
)

it('locates caller import failure at the captured config file', async () => {
  const fixture = adapterFixture()
  try {
    const installed = fixture.install(
      { adapterPath: './missing.cjs' },
      (config) => config,
    )
    await expect(
      installed.adapter.modifyConfig(installed.config, fixture.context),
    ).rejects.toMatchObject({
      location: fixture.session.configFile,
      context: fixture.context,
      stage: 'caller modifyConfig',
    })
  } finally {
    fixture.close()
  }
})

it('retains the original located preparation failure as cause', async () => {
  const fixture = adapterFixture()
  const original = new SyntaxError(
    'content/page.mdx:3:4: required compiler failed',
  )
  try {
    const installed = fixture.install({}, () => {
      throw original
    })
    await expect(
      installed.adapter.modifyConfig(installed.config, fixture.context),
    ).rejects.toMatchObject({
      cause: original,
      context: fixture.context,
      stage: 'finalizer',
    })
  } finally {
    fixture.close()
  }
})

it('rejects a null caller export rather than bypassing its failure', async () => {
  const fixture = adapterFixture()
  try {
    const callerPath = fixture.caller('module.exports = null;')
    const installed = fixture.install(
      { adapterPath: callerPath },
      (config) => config,
    )
    await expect(
      installed.adapter.modifyConfig(installed.config, fixture.context),
    ).rejects.toMatchObject({
      location: fixture.session.configFile,
      stage: 'caller modifyConfig',
    })
  } finally {
    fixture.close()
  }
})

it('bounds a hung caller before invoking the finalizer', async () => {
  const fixture = adapterFixture()
  let finalized = false
  try {
    const callerPath = fixture.caller(
      'module.exports = { modifyConfig() { return new Promise(() => {}) } };',
    )
    const installed = fixture.install(
      { adapterPath: callerPath },
      (config) => {
        finalized = true
        return config
      },
      5,
    )
    await expect(
      installed.adapter.modifyConfig(installed.config, fixture.context),
    ).rejects.toMatchObject({ stage: 'caller modifyConfig' })
    expect(finalized).toBe(false)
  } finally {
    fixture.close()
  }
})

it('bounds a hung finalizer and prevents late completion from publishing', async () => {
  const fixture = adapterFixture()
  const gate = Promise.withResolvers<void>()
  const aborted = Promise.withResolvers<void>()
  let signal: AbortSignal | undefined
  try {
    const installed = fixture.install(
      {},
      async (config, _context, activeSignal) => {
        signal = activeSignal
        activeSignal.addEventListener('abort', () => aborted.resolve(), {
          once: true,
        })
        await gate.promise
        return config
      },
      5,
    )
    const result = installed.adapter.modifyConfig(
      installed.config,
      fixture.context,
    )
    await expect(result).rejects.toMatchObject({
      context: fixture.context,
      stage: 'finalizer',
    })
    await aborted.promise
    expect(signal?.aborted).toBe(true)
    gate.resolve()
  } finally {
    fixture.close()
  }
})

it('release cancels a pending finalizer without affecting another token', async () => {
  const fixture = adapterFixture()
  const other = adapterFixture()
  const entered = Promise.withResolvers<void>()
  const gate = Promise.withResolvers<void>()
  try {
    const installed = fixture.install({}, async (config) => {
      entered.resolve()
      await gate.promise
      return config
    })
    const otherInstalled = other.install({}, (config) => ({
      ...config,
      trailingSlash: true,
    }))
    const pending = installed.adapter.modifyConfig(
      installed.config,
      fixture.context,
    )
    await entered.promise
    installed.release()
    await expect(pending).rejects.toMatchObject({
      stage: 'session release',
      context: fixture.context,
    })
    expect(
      (
        await otherInstalled.adapter.modifyConfig(
          otherInstalled.config,
          other.context,
        )
      ).trailingSlash,
    ).toBe(true)
    gate.resolve()
  } finally {
    fixture.close()
    other.close()
  }
})
