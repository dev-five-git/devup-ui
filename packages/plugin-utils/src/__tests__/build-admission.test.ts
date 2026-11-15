import { createRequire } from 'node:module'

import { expect, it } from 'bun:test'

import admission from '../build-admission.cts'
import { BuildGeneration } from '../build-generation'
import { operation, snapshotEngine } from './generation-fixture'

const owner = { integration: 'Webpack', root: 'owner-root' }
const legacy = { integration: 'Bun', root: 'legacy-root' }

it.each(['reset', 'restore', 'configure', 'action', 'capture'] as const)(
  'releases operation admission when owner %s throws',
  (stage) => {
    const generation = new BuildGeneration<string>()
    const cause = new TypeError(stage)
    let enabled = false
    const fault = (current: string) => {
      if (enabled && current === stage) throw cause
    }
    const engine = snapshotEngine({
      reset() {
        fault('reset')
      },
      restore() {
        fault('restore')
      },
      capture() {
        fault('capture')
        return 'sheet'
      },
    })
    if (stage === 'restore')
      generation.run(
        engine,
        operation(() => undefined),
      )
    enabled = true
    expect(() =>
      admission.runBuildOperation(owner, () =>
        generation.run(
          engine,
          operation(() => {
            fault('configure')
            fault('action')
          }),
        ),
      ),
    ).toThrow(cause)
    let resets = 0
    const end = admission.beginBuild(
      {
        resetBuildState() {
          resets += 1
        },
      },
      legacy,
    )
    end()
    expect(resets).toBe(1)
  },
)

it('restores the enclosing owner when a nested operation releases', () => {
  const inner = { integration: 'NextWebpack', root: 'inner-root' }
  let active
  admission.runBuildOperation(owner, () => {
    admission.runBuildOperation(inner, () => undefined)
    try {
      admission.beginBuild({}, legacy)
    } catch (error) {
      if (!(error instanceof admission.MixedBuildIntegrationError)) throw error
      active = error.active
    }
  })
  expect(active).toEqual(owner)
})

it('keeps the legacy overlap ABI when the last idempotent release ends', () => {
  let resets = 0
  const engine = {
    resetBuildState() {
      resets += 1
    },
  }
  const first = admission.beginBuild(engine, legacy)
  const second = admission.beginBuild(engine, {
    integration: 'Vite',
    root: 'vite-root',
  })
  first()
  first()
  expect(() => admission.runBuildOperation(owner, () => undefined)).toThrow(
    admission.MixedBuildIntegrationError,
  )
  second()
  second()
  admission.beginBuild(engine, legacy)()
  expect(resets).toBe(2)
})

it('shares canonical authority when source, published require and ESM are loaded by Bun', async () => {
  const require = createRequire(import.meta.url)
  const required: typeof import('../index') = require('@devup-ui/plugin-utils')
  const imported = await import('../../dist/index.mjs')
  expect(required.MixedBuildIntegrationError).toBe(
    admission.MixedBuildIntegrationError,
  )
  expect(imported.MixedBuildIntegrationError).toBe(
    admission.MixedBuildIntegrationError,
  )
  const end = imported.beginBuild({}, legacy)
  try {
    expect(() => required.runBuildOperation(owner, () => undefined)).toThrow(
      admission.MixedBuildIntegrationError,
    )
    expect(() => admission.runBuildOperation(owner, () => undefined)).toThrow(
      admission.MixedBuildIntegrationError,
    )
  } finally {
    end()
  }
})
