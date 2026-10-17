import { readFileSync } from 'node:fs'

import { afterEach, expect, it } from 'bun:test'

import { createCore } from '../coordinator-core'
import { parseExtractRequest } from '../coordinator-http'
import { createInput } from '../coordinator-ledger'
import type { ReplayPreparation } from '../coordinator-options'
import { createTestApp, failure, removeTestApps } from './coordinator-app'
import {
  box,
  cssQuery,
  generation,
  prewarmed,
} from './coordinator-prepared-fixture'

afterEach(removeTestApps)

it('rejects unmatched initiating bytes before a refreshed generation is durably adopted', async () => {
  // Given a refreshed disk-first generation but an initiating request with different bytes.
  const app = createTestApp()
  const input = createInput(
    app.root,
    parseExtractRequest(app.post('provider.tsx', box('green'))),
    [],
  )
  const next = { ...generation([]), ordinaryInputs: [input] }
  const prepared = prewarmed(app, {
    generation: { ...generation([]), ordinaryInputs: [] },
    prepareReplay: async ({ generation: current }) =>
      current.ordinaryInputs?.length ? current : next,
  })
  const core = createCore(prepared.options, app.root)
  await core.startup()
  const before = readFileSync(prepared.options.stateFile ?? '', 'utf8')
  try {
    // When unproved initiating bytes request a different ordinary replacement.
    const error = await failure(
      core.extract(parseExtractRequest(app.post('provider.tsx', box('red')))),
    )
    // Then refresh is not committed as an independent partial transaction.
    expect(error).toBeInstanceOf(Error)
    expect(readFileSync(prepared.options.stateFile ?? '', 'utf8')).toBe(before)
    expect((await core.css(cssQuery)).css).toContain('background:green')
  } finally {
    core.close()
  }
})

it('discards ordinary extraction failure after allocation without changing the serving shell', async () => {
  // Given a complete candidate with a valid earlier file and an invalid later one.
  const app = createTestApp()
  const inputs = ['a.tsx', 'z.tsx'].map((filename) =>
    createInput(
      app.root,
      parseExtractRequest(
        app.post(
          filename,
          filename === 'a.tsx'
            ? box('green')
            : "import { css } from '@devup-ui/react'; export const x = css(foo())",
        ),
      ),
      [],
    ),
  )
  const next = { ...generation([]), ordinaryInputs: inputs }
  const prepared = prewarmed(app, {
    generation: { ...generation([]), ordinaryInputs: [] },
    prepareReplay: async () => next,
  })
  const core = createCore(prepared.options, app.root)
  await core.startup()
  const before = readFileSync(prepared.options.stateFile ?? '', 'utf8')
  try {
    // When extraction fails after the candidate has allocated earlier styles.
    const error = await failure(core.css(cssQuery))
    // Then live allocator and disk remain predecessor-exact.
    expect(error).toBeInstanceOf(Error)
    expect(prepared.wasm.exportClassMap()).toBe('{}')
    expect(readFileSync(prepared.options.stateFile ?? '', 'utf8')).toBe(before)
  } finally {
    core.close()
  }
})

it('retires a request-local extraction observer when preparation settles', async () => {
  // Given a provider retaining the observer past its preparation call.
  const app = createTestApp()
  let observer: ReplayPreparation['observeExtraction']
  const prepared = prewarmed(app, {
    generation: generation([]),
    prepareReplay: async ({ generation: current, observeExtraction }) => {
      observer = observeExtraction
      return current
    },
  })
  const core = createCore(prepared.options, app.root)
  await core.startup()
  await core.reconcile()
  if (observer === undefined) throw new Error('missing observer fixture')
  const expired = observer
  try {
    // When provider work tries observing outside the owning call.
    const run = () => expired([], () => undefined)
    // Then the retired call cannot allocate another candidate.
    expect(run).toThrow()
    expect(prepared.extractions).toEqual([])
  } finally {
    core.close()
  }
})
