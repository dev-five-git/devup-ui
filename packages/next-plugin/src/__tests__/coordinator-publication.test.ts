import { readFileSync } from 'node:fs'
import * as fsp from 'node:fs/promises'

import { afterEach, expect, it, spyOn } from 'bun:test'

import { CoordinatorShutdownError } from '../coordinator-completion'
import { createCore } from '../coordinator-core'
import { createReplay } from '../coordinator-replay'
import { CompensationError } from '../coordinator-transaction'
import { createTestApp, failure, removeTestApps } from './coordinator-app'
import {
  box,
  compiledSource,
  generation,
  prewarmed,
} from './coordinator-prepared-fixture'

afterEach(removeTestApps)

function fixture() {
  const app = createTestApp()
  app.write('src/page.mdx', '# raw')
  const initial = {
    ...generation([
      compiledSource(app, { filename: 'src/page.mdx', code: box('red') }),
    ]),
    ordinaryInputs: [],
    plan: { canonicalMap: {}, expectedBaseFiles: ['src/page.mdx'] },
    watchInputs: [app.root],
  }
  const next = {
    ...generation([
      compiledSource(app, { filename: 'src/page.mdx', code: box('green') }),
    ]),
    ordinaryInputs: [],
    plan: {
      canonicalMap: {},
      expectedBaseFiles: ['src/page.mdx', 'missing.tsx'],
    },
    watchInputs: [app.root + '/next'],
  }
  const prepared = prewarmed(app, {
    generation: initial,
    prepareReplay: async () => next,
  })
  return { app, initial, next, prepared }
}

it('acknowledges an atomically published candidate when close wins the commit caller continuation', async () => {
  // Given the actual strong commit wrapped at the reproduced continuation seam.
  const { app, next, prepared } = fixture()
  let watches: readonly string[] = []
  const replay = createReplay(prepared.options, app.root, (inputs) => {
    watches = inputs
  })
  await replay.startup()
  const commit = replay.persistence.commitCandidate
  const observed = spyOn(
    replay.persistence,
    'commitCandidate',
  ).mockImplementation(async (snapshot, previous, control) => {
    await commit(snapshot, previous, control)
    replay.close()
  })
  try {
    // When the owner closes after joined commit acknowledgement, before its caller resumes.
    await replay.reconcile()
    // Then the operation succeeds with candidate CSS and all candidate metadata, never rejected split state.
    expect(replay.live.engine.getCss(0, false)).toContain('background:green')
    expect(replay.snapshot().revision).toBe(8)
    expect(replay.snapshot().plan).toEqual(next.plan)
    expect(replay.ledger.list()[0]?.source).toBe(box('green'))
    expect(replay.live.generation?.configureWasm).toBe(next.configureWasm)
    expect(watches).toEqual(next.watchInputs)
    expect(readFileSync(prepared.options.stateFile ?? '', 'utf8')).toBe(
      JSON.stringify(replay.snapshot()),
    )
    expect(readFileSync(prepared.options.revisionFile ?? '', 'utf8')).toBe('8')
  } finally {
    observed.mockRestore()
    replay.close()
    replay.plan.close()
  }
})

it.each(['throw', 'close'] as const)(
  'retains exact predecessor when the synchronous watch publication callback causes %s',
  async (mode) => {
    // Given a real callback that can fail or revoke the owner before publication.
    const { app, initial, next, prepared } = fixture()
    let watches = initial.watchInputs
    let blocked = true
    const cause = new Error('controlled watch publication failure')
    const replay = createReplay(prepared.options, app.root, (inputs) => {
      watches = [...inputs]
      if (blocked && inputs[0] === next.watchInputs[0]) {
        if (mode === 'throw') throw cause
        replay.close()
      }
    })
    await replay.startup()
    const before = replay.snapshot()
    const output = replay.ledger.lookup('src/page.mdx', box('red'))
    const state = readFileSync(prepared.options.stateFile ?? '', 'utf8')
    const revision = readFileSync(prepared.options.revisionFile ?? '', 'utf8')
    try {
      // When the callback fails after durable writes but before all-state publication.
      const error = await failure(replay.reconcile())
      // Then compensation preserves bytes, allocation, plan, generation and cached output exactly.
      if (mode === 'throw') expect(error).toBe(cause)
      else expect(error).toBeInstanceOf(CoordinatorShutdownError)
      expect(replay.snapshot()).toEqual(before)
      expect(replay.live.generation?.configureWasm).toBe(initial.configureWasm)
      expect(replay.live.engine).toBe(prepared.wasm)
      expect(replay.ledger.lookup('src/page.mdx', box('red'))).toBe(output)
      expect(watches).toEqual(initial.watchInputs)
      expect(readFileSync(prepared.options.stateFile ?? '', 'utf8')).toBe(state)
      expect(readFileSync(prepared.options.revisionFile ?? '', 'utf8')).toBe(
        revision,
      )
      await replay.plan.wait(undefined)
      await replay.persistence.drain()
      blocked = false
      if (mode === 'throw') {
        await replay.reconcile()
        expect(replay.live.revision).toBe(8)
      }
    } finally {
      replay.close()
      replay.plan.close()
    }
  },
)

it('quarantines Core when a watch listener fails both publication and predecessor restoration', async () => {
  // Given the public listener seam whose external state cannot be restored.
  const { app, prepared } = fixture()
  const core = createCore(prepared.options, app.root)
  await core.startup()
  const state = readFileSync(prepared.options.stateFile ?? '', 'utf8')
  const revision = readFileSync(prepared.options.revisionFile ?? '', 'utf8')
  const cause = new Error('watch update failed')
  const compensation = new Error('watch restore failed')
  let calls = 0
  core.onWatchInputs?.(() => {
    throw ++calls === 1 ? cause : compensation
  })
  try {
    // When watch publication and rollback both fail.
    const error = await failure(core.reconcile())
    // Then disk is compensated and no subsequent operation can publish an uncertain owner.
    expect(error).toBeInstanceOf(CompensationError)
    if (
      !(error instanceof CompensationError) ||
      !(error.cause instanceof AggregateError)
    )
      throw new Error('missing compensation causes')
    expect(error.cause.errors).toEqual([cause, compensation])
    expect(readFileSync(prepared.options.stateFile ?? '', 'utf8')).toBe(state)
    expect(readFileSync(prepared.options.revisionFile ?? '', 'utf8')).toBe(
      revision,
    )
    expect(await failure(core.reconcile())).toBe(error)
    expect(await failure(core.flush())).toBe(error)
  } finally {
    core.close()
  }
})

it('keeps live predecessor when cancellation wins the final revision write', async () => {
  // Given a candidate whose real revision rename revokes its lease.
  const { app, prepared } = fixture()
  const replay = createReplay(prepared.options, app.root)
  await replay.startup()
  const before = replay.snapshot()
  const state = readFileSync(prepared.options.stateFile ?? '', 'utf8')
  const revision = readFileSync(prepared.options.revisionFile ?? '', 'utf8')
  const rename = fsp.rename
  const observed = spyOn(fsp, 'rename').mockImplementation(async (from, to) => {
    await rename(from, to)
    if (to === prepared.options.revisionFile) replay.close()
  })
  try {
    // When cancellation wins before the synchronous publication point.
    expect(await failure(replay.reconcile())).toBeInstanceOf(
      CoordinatorShutdownError,
    )
    // Then both IO and live state remain the exact predecessor.
    expect(replay.snapshot()).toEqual(before)
    expect(readFileSync(prepared.options.stateFile ?? '', 'utf8')).toBe(state)
    expect(readFileSync(prepared.options.revisionFile ?? '', 'utf8')).toBe(
      revision,
    )
  } finally {
    observed.mockRestore()
    replay.close()
    replay.plan.close()
  }
})
