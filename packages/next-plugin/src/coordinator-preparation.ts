import { resolve } from 'node:path'

import { CoordinatorShutdownError } from './coordinator-completion'
import { createCore } from './coordinator-core'
import { locatedError } from './coordinator-engine'
import type { CoordinatorStartOptions, Core } from './coordinator-options'
import type { CoordinatorIdentity } from './coordinator-port'
import { type SourceWatcher, watchSources } from './coordinator-watch'

export type PreparationState =
  | { readonly status: 'pending' }
  | { readonly status: 'ready'; readonly core: Core }
  | { readonly status: 'failed'; readonly error: Error }
  | { readonly status: 'cancelled'; readonly error: CoordinatorShutdownError }

export function reportBackgroundError(error: unknown): void {
  console.error(
    '[devup-ui]',
    error instanceof Error ? error.message : String(error),
  )
}

/** Owns the one Core construction, startup and post-startup source watches. */
export function createPreparation(
  options: CoordinatorStartOptions,
  identity: CoordinatorIdentity,
) {
  const controller = new AbortController()
  const settled =
    Promise.withResolvers<Exclude<PreparationState, { status: 'pending' }>>()
  let state: PreparationState = { status: 'pending' }
  let core =
    'prepare' in options ? undefined : createCore(options, identity.project)
  let watcher: SourceWatcher | undefined
  let releaseWatchListener: (() => void) | undefined
  let watchingEnabled = true
  let started = false
  let timer: ReturnType<typeof setTimeout> | undefined

  function stopWatching(): void {
    watchingEnabled = false
    watcher?.close()
    releaseWatchListener?.()
  }

  function finish(
    next: Exclude<PreparationState, { status: 'pending' }>,
  ): void {
    if (controller.signal.aborted) return
    clearTimeout(timer)
    state = next
    settled.resolve(next)
  }

  async function run(): Promise<void> {
    const complete =
      'prepare' in options ? await options.prepare(controller.signal) : options
    if (controller.signal.aborted) return
    if (
      'prepare' in options &&
      (resolve(complete.projectRoot ?? process.cwd()) !==
        resolve(options.projectRoot) ||
        resolve(options.projectRoot, complete.coordinatorPortFile) !==
          resolve(options.projectRoot, options.coordinatorPortFile) ||
        complete.identity?.project !== identity.project ||
        complete.identity.token !== identity.token)
    ) {
      throw locatedError(
        options.coordinatorPortFile,
        'prepare a coordinator',
        'prepared options changed transport ownership',
        'preserve projectRoot, identity and coordinatorPortFile.',
      )
    }
    const active = core ?? createCore(complete, identity.project)
    core = active
    await active.startup()
    if (controller.signal.aborted) return
    if (complete.watch && watchingEnabled) {
      watcher = watchSources({
        roots: (complete.sourceRoots ?? []).map((dir) =>
          resolve(complete.projectRoot ?? process.cwd(), dir),
        ),
        debounceMs: 50,
        onChange: (changedPaths) =>
          void active.reconcile(changedPaths).catch(reportBackgroundError),
        onError: reportBackgroundError,
      })
      watcher.replaceInputs?.(active.watchInputs?.() ?? [])
      releaseWatchListener = active.onWatchInputs?.((inputs) =>
        watcher?.replaceInputs?.(inputs),
      )
    }
    finish({ status: 'ready', core: active })
  }

  return {
    get state(): PreparationState {
      return state
    },
    start(): void {
      if (started || controller.signal.aborted) return
      started = true
      if ('prepare' in options) {
        timer = setTimeout(() => {
          const error = locatedError(
            options.coordinatorPortFile,
            'prepare a coordinator',
            'session preparation exceeded its time budget',
            'finish required preparation before the loader request deadline.',
          )
          finish({ status: 'failed', error })
          controller.abort(error)
          stopWatching()
          core?.close()
        }, options.maxPrepareMs ?? 50_000)
        timer.unref()
      }
      void run().then(undefined, (error: unknown) =>
        finish({
          status: 'failed',
          error: error instanceof Error ? error : new Error(String(error)),
        }),
      )
    },
    async wait(): Promise<Core> {
      const outcome = await settled.promise
      switch (outcome.status) {
        case 'ready':
          controller.signal.throwIfAborted()
          return outcome.core
        case 'failed':
        case 'cancelled':
          throw outcome.error
      }
    },
    close(): PreparationState {
      if (controller.signal.aborted) return state
      const error = new CoordinatorShutdownError('preparation')
      clearTimeout(timer)
      controller.abort(error)
      stopWatching()
      core?.close()
      switch (state.status) {
        case 'pending':
          state = { status: 'cancelled', error }
          settled.resolve(state)
          return state
        case 'ready':
        case 'failed':
        case 'cancelled':
          return state
      }
    },
    stopWatching,
    async flush(): Promise<void> {
      await core?.flush()
    },
  }
}
