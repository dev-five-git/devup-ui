import { existsSync } from 'node:fs'
import * as fsp from 'node:fs/promises'
import { join } from 'node:path'

import { afterEach, describe, expect, it, spyOn } from 'bun:test'

import { resetCoordinator, startCoordinator } from '../coordinator'
import { CoordinatorShutdownError } from '../coordinator-completion'
import { readCoordinatorState } from '../state'
import {
  connect,
  createTestApp,
  failure,
  instrument,
  removeTestApps,
} from './coordinator-app'
import { acceptedRequests, barrier } from './coordinator-barrier-fixture'

afterEach(() => {
  resetCoordinator()
  removeTestApps()
})

describe('deferred cancellation and durability', () => {
  it.each(['close', 'drain'] as const)(
    '%s wakes accepted work and prevents late preparation writes',
    async (operation) => {
      // Given accepted requests waiting on an unresolved preparation callback.
      const { app, pending, entered, handle } = barrier()
      const { engine, extractions } = instrument(app.engine())
      const stateFile = join(app.root, 'df', 'state.json')
      await handle.ready
      const signal = await entered
      const client = connect(app.portFile, app.identity)
      const observed = acceptedRequests(2)
      try {
        const extract = failure(
          client.post('/extract', app.post('a.tsx', 'export const a = 1')),
        )
        const css = failure(client.get('/css?waitForIdle=true'))
        const prepared = failure(handle.prepared)
        await observed.accepted

        // When the owner closes or drains before compilation completes.
        await handle[operation]()

        // Then waiters wake and late completion cannot construct or write Core.
        expect(signal.aborted).toBe(true)
        expect(await prepared).toBeInstanceOf(CoordinatorShutdownError)
        await Promise.all([extract, css])
        pending.resolve(
          app.options({
            wasm: engine,
            stateFile,
            revisionFile: join(app.root, 'df', 'revision'),
            watch: true,
          }),
        )
        await pending.promise
        await Promise.resolve()
        expect(extractions).toEqual([])
        expect(existsSync(stateFile)).toBe(false)
        expect(existsSync(join(app.root, 'df', 'revision'))).toBe(false)
        expect(existsSync(app.portFile)).toBe(false)
      } finally {
        observed.restore()
      }
    },
  )

  it('observes late compiler rejection after cancellation', async () => {
    // Given a compiler promise that ignores cancellation.
    const { pending, entered, handle } = barrier()
    await handle.ready
    await entered
    const prepared = failure(handle.prepared)

    // When drain cancels ownership before the compiler rejects.
    await handle.drain()
    pending.reject(new Error('late compiler failure'))

    // Then preparation remains cancelled without an unhandled rejection.
    expect(await prepared).toBeInstanceOf(CoordinatorShutdownError)
    await Promise.resolve()
  })

  it('does not invoke preparation when closed while ownership transport opens', async () => {
    // Given a deferred coordinator whose listen callback has not run.
    const app = createTestApp()
    let calls = 0
    const handle = startCoordinator({
      projectRoot: app.root,
      identity: app.identity,
      coordinatorPortFile: app.portFile,
      prepare: async () => {
        calls += 1
        return app.options()
      },
    })

    // When close wins transport publication.
    handle.close()
    await handle.ready

    // Then no preparation callback or endpoint publication occurs.
    expect(calls).toBe(0)
    expect(await failure(handle.prepared)).toBeInstanceOf(
      CoordinatorShutdownError,
    )
    expect(existsSync(app.portFile)).toBe(false)
  })

  it('drains startup persistence before owner cleanup and suppresses late watches', async () => {
    // Given startup persistence already accepted but blocked in its rename.
    const { app, pending, handle } = barrier()
    const stateFile = join(app.root, 'df', 'state.json')
    const writing = Promise.withResolvers<void>()
    const release = Promise.withResolvers<void>()
    const rename = fsp.rename
    const spy = spyOn(fsp, 'rename').mockImplementation(async (...args) => {
      writing.resolve()
      await release.promise
      return rename(...args)
    })
    try {
      await handle.ready
      pending.resolve(
        app.options({ stateFile, watch: true, sourceRoots: ['src'] }),
      )
      await writing.promise
      let drained = false

      // When drain cancels preparation during that write.
      const drain = handle.drain().then(() => {
        drained = true
      })
      await Promise.resolve()

      // Then cleanup waits for durability rather than abandoning the write.
      expect(drained).toBe(false)
      expect(existsSync(app.portFile)).toBe(true)
      release.resolve()
      await drain
      expect(readCoordinatorState(stateFile, '')?.inputs).toEqual([])
      expect(existsSync(app.portFile)).toBe(false)
    } finally {
      release.resolve()
      spy.mockRestore()
    }
  })

  it('drains an accepted extraction after preparation without abandoning its snapshot', async () => {
    // Given a prepared coordinator and an accepted extraction write.
    const { app, pending, handle } = barrier()
    const stateFile = join(app.root, 'df', 'state.json')
    pending.resolve(app.options({ stateFile }))
    await handle.ready
    await handle.prepared
    const client = connect(app.portFile, app.identity)
    const writing = Promise.withResolvers<void>()
    const release = Promise.withResolvers<void>()
    const rename = fsp.rename
    const spy = spyOn(fsp, 'rename').mockImplementation(async (...args) => {
      writing.resolve()
      await release.promise
      return rename(...args)
    })
    try {
      const extraction = client.post(
        '/extract',
        app.post('a.tsx', 'export const a = 1'),
      )
      await writing.promise

      // When drain starts while the extraction rename is blocked.
      const drain = handle.drain()
      expect((await client.get('/health')).status).toBe(503)
      release.resolve()

      // Then accepted work remains durable before owner cleanup.
      expect((await extraction).status).toBe(200)
      await drain
      expect(
        readCoordinatorState(stateFile, '')?.inputs.map(
          (input) => input.filename,
        ),
      ).toEqual(['a.tsx'])
      expect(existsSync(app.portFile)).toBe(false)
    } finally {
      release.resolve()
      spy.mockRestore()
    }
  })

  it('joins an in-progress startup write even when preparation times out', async () => {
    // Given startup persistence blocked beyond the preparation deadline.
    const app = createTestApp()
    const stateFile = join(app.root, 'df', 'state.json')
    const writing = Promise.withResolvers<void>()
    const release = Promise.withResolvers<void>()
    const rename = fsp.rename
    const spy = spyOn(fsp, 'rename').mockImplementation(async (...args) => {
      writing.resolve()
      await release.promise
      return rename(...args)
    })
    const handle = startCoordinator({
      projectRoot: app.root,
      identity: app.identity,
      coordinatorPortFile: app.portFile,
      maxPrepareMs: 10,
      prepare: async () => app.options({ stateFile }),
    })
    try {
      await handle.ready
      await writing.promise
      const error = await failure(handle.prepared)
      let drained = false

      // When failed preparation is drained while its startup write is active.
      const drain = failure(handle.drain()).then((result) => {
        drained = true
        return result
      })
      await Promise.resolve()

      // Then the failure propagates only after write draining and owner cleanup.
      expect(drained).toBe(false)
      expect(existsSync(app.portFile)).toBe(true)
      release.resolve()
      expect(await drain).toBe(error)
      expect(existsSync(app.portFile)).toBe(false)
      expect(readCoordinatorState(stateFile, '')?.inputs).toEqual([])
    } finally {
      release.resolve()
      spy.mockRestore()
    }
  })
})
