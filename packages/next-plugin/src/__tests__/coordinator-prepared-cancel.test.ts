import { existsSync, readFileSync } from 'node:fs'
import * as fsp from 'node:fs/promises'
import { join } from 'node:path'

import { afterEach, expect, it, spyOn } from 'bun:test'

import { CoordinatorShutdownError } from '../coordinator-completion'
import { createCore } from '../coordinator-core'
import type { PreparedSourceGeneration } from '../coordinator-options'
import { readCoordinatorState } from '../state'
import { createTestApp, failure, removeTestApps } from './coordinator-app'
import {
  box,
  compiledSource,
  cssQuery,
  generation,
  preparedCore,
  prewarmed,
} from './coordinator-prepared-fixture'

afterEach(removeTestApps)

it('joins a late generation after owner cancellation without replay or publication', async () => {
  // Given a required refresh whose controlled provider ignores cancellation until released.
  const app = createTestApp()
  app.write('src/page.mdx', '# old')
  const entered = Promise.withResolvers<AbortSignal>()
  const pending = Promise.withResolvers<PreparedSourceGeneration>()
  const prepared = await preparedCore(app, {
    generation: generation([
      compiledSource(app, { filename: 'src/page.mdx', code: box('red') }),
    ]),
    prepareReplay: ({ signal }) => {
      entered.resolve(signal)
      return pending.promise
    },
  })
  const before = readFileSync(prepared.options.stateFile ?? '', 'utf8')
  const refreshing = failure(prepared.core.css(cssQuery))
  const signal = await entered.promise
  let joined = false

  // When the owner closes before the late provider resolves.
  prepared.core.close()
  const flush = prepared.core.flush().then(() => {
    joined = true
  })
  await Promise.resolve()
  expect(joined).toBe(false)
  pending.resolve(
    generation([
      compiledSource(app, { filename: 'src/page.mdx', code: box('green') }),
    ]),
  )

  // Then shutdown joins the provider, but late compiled bytes never initialize a candidate or publish.
  expect(await refreshing).toBeInstanceOf(CoordinatorShutdownError)
  await flush
  expect(signal.aborted).toBe(true)
  expect(readFileSync(prepared.options.stateFile ?? '', 'utf8')).toBe(before)
  expect(prepared.extractions).toEqual(['map:src/page.mdx'])
})

it('prevents prepared startup writes when the owner closes before its queued initialization', async () => {
  // Given a fully prepared engine but no started Core mutation.
  const app = createTestApp()
  const prepared = prewarmed(app, { generation: generation([]) })
  const core = createCore(prepared.options, app.root)

  // When close wins the startup mutation queue.
  const startup = failure(core.startup())
  core.close()

  // Then late initialization is rejected and cannot create a checkpoint.
  expect(await startup).toBeInstanceOf(CoordinatorShutdownError)
  await core.flush()
  expect(existsSync(prepared.options.stateFile ?? '')).toBe(false)
})

it('drains writes already accepted before cancellation', async () => {
  // Given an accepted ordinary extraction paused at the real atomic rename boundary.
  const app = createTestApp()
  const prepared = await preparedCore(app, { generation: generation([]) })
  const entered = Promise.withResolvers<void>()
  const released = Promise.withResolvers<void>()
  const rename = fsp.rename
  const observed = spyOn(fsp, 'rename').mockImplementation(async (from, to) => {
    if (to === prepared.options.stateFile) {
      entered.resolve()
      await released.promise
    }
    return rename(from, to)
  })
  try {
    const accepted = prepared.core.extract({
      filename: 'ordinary.tsx',
      resourcePath: join(app.root, 'ordinary.tsx'),
      code: box('blue'),
    })
    await entered.promise
    let drained = false

    // When the owner closes while the already accepted write is pending.
    prepared.core.close()
    const flush = prepared.core.flush().then(() => {
      drained = true
    })
    await Promise.resolve()
    expect(drained).toBe(false)
    released.resolve()

    // Then accepted writes still finish and drain records their exact input.
    expect((await accepted).code).toContain('className')
    await flush
    expect(
      readCoordinatorState(prepared.options.stateFile ?? '', '')?.inputs[0]
        ?.source,
    ).toBe(box('blue'))
  } finally {
    observed.mockRestore()
  }
})

it('cancels live publication while an already submitted candidate write drains', async () => {
  // Given a candidate replay paused at its accepted checkpoint write.
  const app = createTestApp()
  app.write('src/page.mdx', '# raw')
  const next = generation([
    compiledSource(app, { filename: 'src/page.mdx', code: box('green') }),
  ])
  const prepared = await preparedCore(app, {
    generation: generation([
      compiledSource(app, { filename: 'src/page.mdx', code: box('red') }),
    ]),
    prepareReplay: async () => next,
  })
  const entered = Promise.withResolvers<void>()
  const released = Promise.withResolvers<void>()
  const rename = fsp.rename
  const observed = spyOn(fsp, 'rename').mockImplementation(async (from, to) => {
    if (to === prepared.options.stateFile) {
      entered.resolve()
      await released.promise
    }
    return rename(from, to)
  })
  try {
    const refreshing = failure(prepared.core.css(cssQuery))
    await entered.promise

    // When cancellation wins candidate publication, but not its already accepted disk write.
    prepared.core.close()
    released.resolve()

    // Then the write drains, the request rejects, and the original serving engine is unchanged.
    expect(await refreshing).toBeInstanceOf(CoordinatorShutdownError)
    await prepared.core.flush()
    expect(prepared.wasm.getCss(0, false)).toContain('background:red')
    expect(
      readCoordinatorState(prepared.options.stateFile ?? '', '')?.inputs[0]
        ?.source,
    ).toBe(box('green'))
  } finally {
    observed.mockRestore()
  }
})
