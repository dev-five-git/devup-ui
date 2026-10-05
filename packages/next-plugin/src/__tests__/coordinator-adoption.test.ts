import { readFileSync } from 'node:fs'
import * as fsp from 'node:fs/promises'
import { join } from 'node:path'

import { afterEach, expect, it, spyOn } from 'bun:test'

import { createCore } from '../coordinator-core'
import { immutableGeneration } from '../coordinator-generation'
import { parseExtractRequest } from '../coordinator-http'
import { createInput, stampFile } from '../coordinator-ledger'
import type { ReplayExtractionReport } from '../coordinator-options'
import { readCoordinatorState } from '../state'
import { createTestApp, failure, removeTestApps } from './coordinator-app'
import {
  box,
  cssQuery,
  generation,
  prewarmed,
} from './coordinator-prepared-fixture'

afterEach(removeTestApps)

it('retains detached ordinary plan and watch metadata when copying a generation', () => {
  // Given mutable provider data and one resolver identity.
  const app = createTestApp()
  const input = createInput(
    app.root,
    parseExtractRequest(app.post('provider.tsx', box('blue'))),
    [],
  )
  const source = {
    ...generation([]),
    ordinaryInputs: [input],
    plan: {
      canonicalMap: { 'provider.tsx': '@global' },
      expectedBaseFiles: ['provider.tsx'],
    },
    watchInputs: [join(app.root, 'outside.json')],
  }
  // When Core copies data and the provider mutates its containers.
  const copied = immutableGeneration(source)
  source.ordinaryInputs.splice(0)
  source.plan.expectedBaseFiles.splice(0)
  source.watchInputs.splice(0)
  // Then retained metadata is immutable without changing the resolver identity.
  expect(copied.ordinaryInputs).toEqual([input])
  expect(copied.plan?.expectedBaseFiles).toEqual(['provider.tsx'])
  expect(copied.watchInputs).toEqual([join(app.root, 'outside.json')])
  expect(Object.isFrozen(copied.plan?.canonicalMap)).toBe(true)
  expect(copied.configureWasm).toBe(source.configureWasm)
})

it('adopts new ordinary bytes before POST and makes cache-zero resend a durable no-op', async () => {
  // Given a new proved disk-first generation and no output-cache capacity.
  const app = createTestApp()
  app.write('provider.tsx', box('blue'))
  const input = createInput(
    app.root,
    parseExtractRequest(app.post('provider.tsx')),
    [],
  )
  const next = {
    ...generation([]),
    ordinaryInputs: [input],
    watchInputs: [input.resourcePath],
  }
  const prepared = prewarmed(app, {
    generation: { ...generation([]), ordinaryInputs: [] },
    prepareReplay: async ({ generation: current }) =>
      current.ordinaryInputs?.length ? current : next,
  })
  const core = createCore({ ...prepared.options, cacheMaxEntries: 0 }, app.root)
  await core.startup()
  // When CSS prepares the provider before the identical native resend.
  const css = await core.css(cssQuery)
  const checkpoint = readFileSync(prepared.options.stateFile ?? '', 'utf8')
  const response = await core.extract(
    parseExtractRequest(app.post('provider.tsx')),
  )
  // Then actual CSS/output exists and resend makes no new durable revision.
  expect(css.css).toContain('background:blue')
  expect(response.code).toContain('className')
  expect(readFileSync(prepared.options.stateFile ?? '', 'utf8')).toBe(
    checkpoint,
  )
  expect(core.watchInputs?.()).toEqual([input.resourcePath])
  core.close()
})

it('keeps omitted ordinary survivors when replacement overlays changed bytes', async () => {
  // Given ordinary ownership separate from the compiled set.
  const app = createTestApp()
  const old = createInput(
    app.root,
    parseExtractRequest(app.post('provider.tsx', box('red'))),
    [],
  )
  const survivor = createInput(
    app.root,
    parseExtractRequest(app.post('survivor.tsx', box('blue'))),
    [],
  )
  const replacement = { ...old, source: box('green') }
  const next = { ...generation([]), ordinaryInputs: [replacement] }
  const prepared = prewarmed(app, {
    ordinaryInputs: [old, survivor],
    generation: { ...generation([]), ordinaryInputs: [old, survivor] },
    prepareReplay: async ({ generation: current }) =>
      current.ordinaryInputs?.[0]?.source === box('green') ? current : next,
  })
  const core = createCore(prepared.options, app.root)
  await core.startup()
  // When a complete ordinary reach projection omits a still-existing survivor.
  await core.reconcile()
  // Then omission is not pruning, and replacement is durable.
  expect(
    readCoordinatorState(prepared.options.stateFile ?? '', '')?.inputs.map(
      ({ source }) => source,
    ),
  ).toEqual([box('green'), box('blue')])
  core.close()
})

it('compensates checkpoint and revision when cancellation wins after candidate IO', async () => {
  // Given an opted-in candidate paused at atomic state rename.
  const app = createTestApp()
  const next = {
    ...generation([]),
    ordinaryInputs: [
      createInput(
        app.root,
        parseExtractRequest(app.post('provider.tsx', box('green'))),
        [],
      ),
    ],
  }
  const prepared = prewarmed(app, {
    generation: { ...generation([]), ordinaryInputs: [] },
    prepareReplay: async () => next,
  })
  const core = createCore(prepared.options, app.root)
  await core.startup()
  const state = readFileSync(prepared.options.stateFile ?? '', 'utf8')
  const revision = readFileSync(prepared.options.revisionFile ?? '', 'utf8')
  const entered = Promise.withResolvers<void>()
  const released = Promise.withResolvers<void>()
  const rename = fsp.rename
  let paused = false
  const spy = spyOn(fsp, 'rename').mockImplementation(async (from, to) => {
    if (to === prepared.options.stateFile && !paused) {
      paused = true
      entered.resolve()
      await released.promise
    }
    return rename(from, to)
  })
  try {
    const refreshing = failure(core.css(cssQuery))
    await entered.promise
    // When close wins before candidate publication.
    core.close()
    released.resolve()
    const error = await refreshing
    await core.flush()
    // Then compensation joins IO and restores both exact durable predecessors.
    expect(error).toBeInstanceOf(Error)
    expect(readFileSync(prepared.options.stateFile ?? '', 'utf8')).toBe(state)
    expect(readFileSync(prepared.options.revisionFile ?? '', 'utf8')).toBe(
      revision,
    )
  } finally {
    spy.mockRestore()
  }
})

it('validates the adopted production generation at every CSS publication', async () => {
  // Given a prepared nonwatch owner whose reported input changes after initial publication.
  const app = createTestApp()
  const dependency = app.write('outside.json', 'prepared input')
  const fingerprint = stampFile(dependency)
  const prepared = prewarmed(app, {
    generation: { ...generation([]), watchInputs: [dependency] },
  })
  const provider = prepared.options.preparedSources
  if (provider === undefined) throw new Error('missing fixture provider')
  const observed: boolean[] = []
  const core = createCore(
    {
      ...prepared.options,
      watch: false,
      preparedSources: {
        ...provider,
        validateForCssFinalization(current) {
          observed.push(Object.isFrozen(current))
          if (stampFile(dependency) !== fingerprint)
            throw new Error('outside.json:1:1: changed reported input')
        },
      },
    },
    app.root,
  )
  await core.startup()
  await core.css(cssQuery)
  app.write('outside.json', 'changed input')
  // When a second production publication tries to capture CSS.
  const error = await failure(core.css(cssQuery))
  // Then freshness rejects even after sealing, without starting dev preparation.
  expect(error).toBeInstanceOf(Error)
  expect(observed).toEqual([true, true])
  core.close()
})

it('observes complete inputs on isolated engines with the supplied resolver', async () => {
  // Given request-local observation with a resolver different from the live one.
  const app = createTestApp()
  const input = createInput(
    app.root,
    parseExtractRequest(app.post('provider.tsx', box('green'))),
    [],
  )
  let reports: readonly ReplayExtractionReport[] = []
  const prepared = prewarmed(app, {
    generation: { ...generation([]), ordinaryInputs: [] },
    prepareReplay: async ({
      generation: current,
      observeExtraction,
      changedPaths,
    }) => {
      if (observeExtraction === undefined) throw new Error('missing observer')
      expect(changedPaths).toEqual([join(app.root, 'changed.json')])
      reports = observeExtraction([input], (wasm) => {
        expect(wasm).not.toBe(prepared.wasm)
      })
      expect(prepared.wasm.getCss(0, false)).not.toContain('background:green')
      return current
    },
  })
  const core = createCore(prepared.options, app.root)
  await core.startup()
  // When refresh observes a complete candidate using its own configurer.
  await core.reconcile([join(app.root, 'changed.json')])
  // Then observation did not mutate live state or revision.
  expect(reports).toEqual([{ filename: 'provider.tsx', dependencies: [] }])
  expect(
    readCoordinatorState(prepared.options.stateFile ?? '', '')?.revision,
  ).toBe(7)
  core.close()
})
