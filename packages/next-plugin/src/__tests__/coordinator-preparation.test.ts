import { existsSync } from 'node:fs'
import { join } from 'node:path'

import { afterEach, describe, expect, it, spyOn } from 'bun:test'

import { CoordinatorShutdownError } from '../coordinator-completion'
import type { CoordinatorOptions } from '../coordinator-options'
import {
  createPreparation,
  reportBackgroundError,
} from '../coordinator-preparation'
import { createTestApp, failure, removeTestApps } from './coordinator-app'

afterEach(removeTestApps)

describe('preparation outcomes', () => {
  it('starts once and retains the ready Core for prepared callers', async () => {
    // Given complete prepared options and a pending startup outcome.
    const app = createTestApp()
    const preparation = createPreparation(app.options(), app.identity)
    expect(preparation.state).toEqual({ status: 'pending' })

    // When startup is requested twice.
    preparation.start()
    preparation.start()
    const core = await preparation.wait()

    // Then both callers receive the same Core and close gates later work.
    expect(preparation.state).toEqual({ status: 'ready', core })
    expect(await preparation.wait()).toBe(core)
    preparation.close()
    preparation.close()
    expect(await failure(preparation.wait())).toBeInstanceOf(
      CoordinatorShutdownError,
    )
    await preparation.flush()
  })

  it('settles cancellation before startup and never invokes the callback', async () => {
    // Given preparation that has not started.
    const app = createTestApp()
    let calls = 0
    const preparation = createPreparation(
      {
        projectRoot: app.root,
        identity: app.identity,
        coordinatorPortFile: app.portFile,
        prepare: async () => {
          calls += 1
          return app.options()
        },
      },
      app.identity,
    )

    // When cancellation wins startup.
    preparation.close()
    preparation.start()

    // Then waiters receive cancellation without invoking the callback.
    expect(preparation.state.status).toBe('cancelled')
    expect(await failure(preparation.wait())).toBeInstanceOf(
      CoordinatorShutdownError,
    )
    expect(calls).toBe(0)
    await preparation.flush()
  })

  it('normalizes a non-Error rejection into the retained failed outcome', async () => {
    // Given a compiler callback rejecting with a string.
    const app = createTestApp()
    const preparation = createPreparation(
      {
        projectRoot: app.root,
        identity: app.identity,
        coordinatorPortFile: app.portFile,
        prepare: () => Promise.reject('compiler stopped'),
      },
      app.identity,
    )

    // When preparation runs that callback.
    preparation.start()
    const error = await failure(preparation.wait())

    // Then failure is retained as a typed Error, including after close.
    if (!(error instanceof Error)) throw error
    expect(error).toBeInstanceOf(Error)
    expect(String(error)).toContain('compiler stopped')
    expect(preparation.state).toEqual({ status: 'failed', error })
    preparation.close()
    expect(await failure(preparation.wait())).toBe(error)
  })

  it.each(['projectRoot', 'coordinatorPortFile', 'identity'] as const)(
    'rejects changed %s before constructing or writing Core',
    async (field) => {
      // Given prepared options replacing one ownership field.
      const app = createTestApp()
      const stateFile = join(app.root, 'df', 'state.json')
      const complete = app.options({ stateFile })
      const changed = {
        projectRoot: { ...complete, projectRoot: join(app.root, 'foreign') },
        coordinatorPortFile: {
          ...complete,
          coordinatorPortFile: join(app.root, 'foreign'),
        },
        identity: {
          ...complete,
          identity: { ...app.identity, token: 'foreign' },
        },
      }[field]
      const preparation = createPreparation(
        {
          projectRoot: app.root,
          identity: app.identity,
          coordinatorPortFile: app.portFile,
          prepare: async () => changed,
        },
        app.identity,
      )

      // When preparation receives those options.
      preparation.start()
      const error = await failure(preparation.wait())

      // Then ownership validation fails before Core writes.
      expect(String(error)).toContain(
        'prepared options changed transport ownership',
      )
      expect(existsSync(stateFile)).toBe(false)
      preparation.close()
    },
  )

  it('bounds a hung preparation and prevents late construction after its deadline', async () => {
    // Given a callback that ignores its bounded cancellation signal.
    const app = createTestApp()
    const pending = Promise.withResolvers<CoordinatorOptions>()
    const stateFile = join(app.root, 'df', 'state.json')
    const entered = Promise.withResolvers<AbortSignal>()
    const preparation = createPreparation(
      {
        projectRoot: app.root,
        identity: app.identity,
        coordinatorPortFile: app.portFile,
        maxPrepareMs: 1,
        prepare: (signal) => {
          entered.resolve(signal)
          return pending.promise
        },
      },
      app.identity,
    )

    // When its preparation deadline expires.
    preparation.start()
    const error = await failure(preparation.wait())

    // Then the retained deadline failure also prevents late Core construction.
    if (!(error instanceof Error)) throw error
    expect(preparation.state).toEqual({ status: 'failed', error })
    expect((await entered.promise).aborted).toBe(true)
    expect(String(error)).toContain('preparation exceeded its time budget')
    pending.resolve(app.options({ stateFile }))
    await pending.promise
    await preparation.flush()
    expect(existsSync(stateFile)).toBe(false)
    preparation.close()
  })

  it('reports non-Error background failures without dropping their diagnostic', () => {
    // Given a non-Error diagnostic at the reporting boundary.
    const reporting = spyOn(console, 'error')
    try {
      // When it is reported through the existing project logger.
      reportBackgroundError('compiler diagnostic')

      // Then the diagnostic reaches that logger unchanged.
      expect(reporting).toHaveBeenCalledWith(
        '[devup-ui]',
        'compiler diagnostic',
      )
    } finally {
      reporting.mockRestore()
    }
  })
})
