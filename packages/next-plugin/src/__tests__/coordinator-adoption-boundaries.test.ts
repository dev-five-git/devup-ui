import { existsSync, mkdirSync, readFileSync, rmSync } from 'node:fs'
import * as fsp from 'node:fs/promises'

import { afterEach, expect, it, spyOn } from 'bun:test'

import { createCore } from '../coordinator-core'
import { parseExtractRequest } from '../coordinator-http'
import { createInput } from '../coordinator-ledger'
import { CompensationError } from '../coordinator-transaction'
import { createTestApp, failure, removeTestApps } from './coordinator-app'
import {
  box,
  compiledSource,
  cssQuery,
  generation,
  prewarmed,
} from './coordinator-prepared-fixture'

afterEach(removeTestApps)

it('refuses duplicate compiled candidates while retaining the serving predecessor', async () => {
  // Given a valid predecessor and two competing compiled replacements.
  const app = createTestApp()
  const old = compiledSource(app, {
    filename: 'provider.tsx',
    code: box('red'),
  })
  const candidate = compiledSource(app, {
    filename: 'provider.tsx',
    code: box('green'),
  })
  let duplicates = true
  const prepared = prewarmed(app, {
    generation: { ...generation([old]), ordinaryInputs: [] },
    prepareReplay: async ({ generation: current }) =>
      duplicates
        ? { ...generation([candidate, candidate]), ordinaryInputs: [] }
        : current,
  })
  const core = createCore(prepared.options, app.root)
  await core.startup()
  const state = readFileSync(prepared.options.stateFile ?? '', 'utf8')
  const revision = readFileSync(prepared.options.revisionFile ?? '', 'utf8')
  const allocations = prepared.wasm.exportClassMap()
  try {
    // When refresh attempts ambiguous compiled ownership.
    const error = await failure(core.reconcile())
    duplicates = false
    // Then no engine, checkpoint, revision or serving CSS changes.
    expect(error).toBeInstanceOf(Error)
    expect(readFileSync(prepared.options.stateFile ?? '', 'utf8')).toBe(state)
    expect(readFileSync(prepared.options.revisionFile ?? '', 'utf8')).toBe(
      revision,
    )
    expect(prepared.wasm.exportClassMap()).toBe(allocations)
    expect((await core.css(cssQuery)).css).toContain('background:red')
    expect((await core.css(cssQuery)).css).not.toContain('background:green')
  } finally {
    core.close()
  }
})

it.each([false, true])(
  'restores checkpoint absence after failed adoption (failed removal: %s)',
  async (removalFails) => {
    // Given an opted-in owner whose predecessor checkpoint is absent.
    const app = createTestApp()
    const input = createInput(
      app.root,
      parseExtractRequest(app.post('provider.tsx', box('green'))),
      [],
    )
    let candidate = true
    const prepared = prewarmed(app, {
      generation: { ...generation([]), ordinaryInputs: [] },
      prepareReplay: async ({ generation: current }) =>
        candidate ? { ...generation([]), ordinaryInputs: [input] } : current,
    })
    const core = createCore(prepared.options, app.root)
    await core.startup()
    const stateFile = prepared.options.stateFile ?? ''
    const revisionFile = prepared.options.revisionFile ?? ''
    rmSync(stateFile)
    const revision = readFileSync(revisionFile, 'utf8')
    const rename = fsp.rename
    const rm = fsp.rm
    const publicationFailure = new Error('controlled revision failure')
    const removalFailure = new Error('controlled compensation removal failure')
    const renameSpy = spyOn(fsp, 'rename').mockImplementation((from, to) => {
      if (to === revisionFile) return Promise.reject(publicationFailure)
      return rename(from, to)
    })
    const rmSpy = spyOn(fsp, 'rm').mockImplementation((path, options) => {
      if (removalFails && path === stateFile)
        return Promise.reject(removalFailure)
      return rm(path, options)
    })
    try {
      // When revision publication fails after creating a candidate checkpoint.
      const error = await failure(core.css(cssQuery))
      candidate = false
      // Then absence is compensated, or failed compensation quarantines the owner.
      expect(readFileSync(revisionFile, 'utf8')).toBe(revision)
      if (removalFails) {
        expect(error).toBeInstanceOf(CompensationError)
        if (!(error instanceof CompensationError))
          throw new Error('expected failed compensation')
        expect(error.cause).toBeInstanceOf(AggregateError)
        if (!(error.cause instanceof AggregateError))
          throw new Error('expected original IO failures')
        expect(error.cause.errors).toEqual([publicationFailure, removalFailure])
        expect(await failure(core.css(cssQuery))).toBe(error)
        expect(await failure(core.flush())).toBe(error)
      } else {
        expect(error).toBe(publicationFailure)
        expect(existsSync(stateFile)).toBe(false)
        expect((await core.css(cssQuery)).css).not.toContain('background:green')
        await core.flush()
      }
    } finally {
      rmSpy.mockRestore()
      renameSpy.mockRestore()
      core.close()
    }
  },
)

it('refuses adoption when the predecessor checkpoint cannot be read', async () => {
  // Given an opted-in owner with a directory obstructing its checkpoint.
  const app = createTestApp()
  const input = createInput(
    app.root,
    parseExtractRequest(app.post('provider.tsx', box('green'))),
    [],
  )
  let candidate = true
  const prepared = prewarmed(app, {
    generation: { ...generation([]), ordinaryInputs: [] },
    prepareReplay: async ({ generation: current }) =>
      candidate ? { ...generation([]), ordinaryInputs: [input] } : current,
  })
  const core = createCore(prepared.options, app.root)
  await core.startup()
  const stateFile = prepared.options.stateFile ?? ''
  const revisionFile = prepared.options.revisionFile ?? ''
  rmSync(stateFile)
  mkdirSync(stateFile)
  const sentinel = app.write('df/snapshot.json/predecessor', 'keep me')
  const revision = readFileSync(revisionFile, 'utf8')
  try {
    // When the transaction cannot capture the exact predecessor.
    const error = await failure(core.css(cssQuery))
    candidate = false
    // Then it propagates the IO error before writes and retains the serving shell.
    expect(error).toBeInstanceOf(Error)
    expect(readFileSync(sentinel, 'utf8')).toBe('keep me')
    expect(readFileSync(revisionFile, 'utf8')).toBe(revision)
    expect((await core.css(cssQuery)).css).not.toContain('background:green')
    await core.flush()
  } finally {
    core.close()
  }
})
