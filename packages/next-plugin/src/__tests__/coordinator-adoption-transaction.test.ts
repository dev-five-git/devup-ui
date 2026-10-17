import { existsSync, readFileSync, rmSync } from 'node:fs'
import * as fsp from 'node:fs/promises'

import { afterEach, expect, it, spyOn } from 'bun:test'

import { createCore } from '../coordinator-core'
import { parseExtractRequest } from '../coordinator-http'
import { createInput } from '../coordinator-ledger'
import { CompensationError } from '../coordinator-transaction'
import { readCoordinatorState } from '../state'
import { createTestApp, failure, removeTestApps } from './coordinator-app'
import {
  box,
  cssQuery,
  generation,
  prewarmed,
} from './coordinator-prepared-fixture'

afterEach(removeTestApps)

it('rejects overlapping ordinary and compiled ownership without publication', async () => {
  // Given a provider attempting to classify one input as ordinary and compiled.
  const app = createTestApp()
  const input = createInput(
    app.root,
    parseExtractRequest(app.post('provider.tsx', box('red'))),
    [],
  )
  const next = {
    ...generation([
      {
        input,
        evidence: {
          compilerFingerprint: 'fixture',
          fileFingerprints: {},
          contextFingerprints: {},
          missingDependencies: [],
        },
      },
    ]),
    ordinaryInputs: [input],
  }
  const prepared = prewarmed(app, {
    generation: { ...generation([]), ordinaryInputs: [] },
    prepareReplay: async () => next,
  })
  const core = createCore(prepared.options, app.root)
  await core.startup()
  const before = readFileSync(prepared.options.stateFile ?? '', 'utf8')
  // When refresh tries adopting conflicting ownership.
  const error = await failure(core.reconcile())
  // Then the error blocks both CSS and durable publication.
  expect(error).toBeInstanceOf(Error)
  expect(readFileSync(prepared.options.stateFile ?? '', 'utf8')).toBe(before)
  core.close()
})

it.each(['stateFile', 'revisionFile'])(
  'restores the predecessor after ordinary adoption %s failure and allows corrected retry',
  async (destination) => {
    // Given an opted-in owner and a candidate containing new ordinary styles.
    const app = createTestApp()
    const input = createInput(
      app.root,
      parseExtractRequest(app.post('provider.tsx', box('green'))),
      [],
    )
    const next = {
      ...generation([]),
      ordinaryInputs: [input],
      plan: { canonicalMap: {}, expectedBaseFiles: ['provider.tsx'] },
    }
    const prepared = prewarmed(app, {
      generation: { ...generation([]), ordinaryInputs: [] },
      prepareReplay: async ({ generation: current }) =>
        current.ordinaryInputs?.length ? current : next,
    })
    const core = createCore(prepared.options, app.root)
    await core.startup()
    const before = readFileSync(prepared.options.stateFile ?? '', 'utf8')
    const revision = readFileSync(prepared.options.revisionFile ?? '', 'utf8')
    const rename = fsp.rename
    let blocked = true
    const spy = spyOn(fsp, 'rename').mockImplementation(async (from, to) => {
      if (blocked && to === prepared.options[destination])
        throw new Error('controlled candidate write failure')
      return rename(from, to)
    })
    try {
      // When candidate durability fails before adoption.
      const error = await failure(core.css(cssQuery))
      // Then old ledger/revision/plan remain exact and corrected retry commits once.
      expect(error).toBeInstanceOf(Error)
      expect(readFileSync(prepared.options.stateFile ?? '', 'utf8')).toBe(
        before,
      )
      expect(readFileSync(prepared.options.revisionFile ?? '', 'utf8')).toBe(
        revision,
      )
      expect(core.watchInputs?.()).toEqual([])
      blocked = false
      expect((await core.css(cssQuery)).css).toContain('background:green')
      expect(
        readCoordinatorState(prepared.options.stateFile ?? '', '')?.revision,
      ).toBe(8)
      expect(
        readCoordinatorState(prepared.options.stateFile ?? '', '')?.plan,
      ).toEqual(next.plan)
    } finally {
      spy.mockRestore()
      core.close()
    }
  },
)

it('restores prior revision absence when cancellation follows revision publication', async () => {
  // Given an opted-in owner with no predecessor revision marker.
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
  const before = readFileSync(prepared.options.stateFile ?? '', 'utf8')
  rmSync(prepared.options.revisionFile ?? '')
  const rename = fsp.rename
  const spy = spyOn(fsp, 'rename').mockImplementation(async (from, to) => {
    await rename(from, to)
    if (to === prepared.options.revisionFile) core.close()
  })
  try {
    // When close wins immediately after revision rename.
    const error = await failure(core.css(cssQuery))
    await core.flush()
    // Then checkpoint bytes and marker absence are restored exactly.
    expect(error).toBeInstanceOf(Error)
    expect(readFileSync(prepared.options.stateFile ?? '', 'utf8')).toBe(before)
    expect(existsSync(prepared.options.revisionFile ?? '')).toBe(false)
  } finally {
    spy.mockRestore()
  }
})

it('quarantines publication when predecessor compensation fails', async () => {
  // Given a revision failure followed by a checkpoint-compensation failure.
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
  const rename = fsp.rename
  let stateWrites = 0
  const spy = spyOn(fsp, 'rename').mockImplementation(async (from, to) => {
    if (to === prepared.options.stateFile && ++stateWrites > 1)
      throw new Error('compensation failed')
    if (to === prepared.options.revisionFile) throw new Error('revision failed')
    return rename(from, to)
  })
  try {
    // When required compensation cannot restore the durable predecessor.
    const error = await failure(core.css(cssQuery))
    // Then further publication/drain fails rather than asserting restoration.
    expect(error).toBeInstanceOf(CompensationError)
    expect(await failure(core.css(cssQuery))).toBe(error)
    expect(await failure(core.flush())).toBe(error)
  } finally {
    spy.mockRestore()
    core.close()
  }
})
