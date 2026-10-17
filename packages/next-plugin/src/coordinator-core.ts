import {
  type ExtractOutputSnapshot,
  extractRequest,
  locatedError,
  toExtractResponse,
} from './coordinator-engine'
import { assertAdoptedRequest, preparedInput } from './coordinator-generation'
import { type ExtractRequest, HttpError } from './coordinator-http'
import { createInput } from './coordinator-ledger'
import type { CoordinatorOptions, Core } from './coordinator-options'
import { createReplay } from './coordinator-replay'
import { extractSealed } from './coordinator-sealed'
import { elapsedMs, profileStart, reportProfile } from './profile'

interface Accepted {
  output: ExtractOutputSnapshot
  committed: Promise<void>
  cacheHit: boolean
}

/**
 * Everything a coordinator knows, with no HTTP in it: the engine in service,
 * the inputs it holds, completion tracking and persistence. Every change goes
 * through one queue, so the engine is never mutated by two requests at once.
 */
export function createCore(options: CoordinatorOptions, project: string): Core {
  let watchListener: ((inputs: readonly string[]) => void) | undefined
  const replay = createReplay(options, project, (inputs) =>
    watchListener?.(inputs),
  )
  const {
    root,
    settings,
    createEngine,
    configure,
    ledger,
    plan,
    persistence,
    live,
    snapshot: liveSnapshot,
    reconcile,
  } = replay
  const watch = options.watch ?? false
  const watchInputs = () => [
    ...(live.generation?.watchInputs ?? []),
    ...ledger.resolutionInputs().map((input) => input.path),
  ]
  let sealed = false

  let queue: Promise<unknown> = Promise.resolve()
  function mutate<T>(task: () => Promise<T>): Promise<T> {
    const run = queue.then(() => {
      persistence.assertHealthy()
      return task()
    })
    // The caller sees a failure through `run`; the queue just keeps going.
    queue = run.then(
      () => undefined,
      () => undefined,
    )
    return run
  }

  async function accept(request: ExtractRequest): Promise<Accepted> {
    await reconcile(request)
    const prepared = preparedInput(live.generation, request)
    if (live.generation?.ordinaryInputs !== undefined) {
      await persistence.drain()
      assertAdoptedRequest(ledger.list(), request)
      const output =
        ledger.lookup(request.filename, request.code, request.sourceType) ??
        extractSealed(
          { live: live.engine, createEngine, configure, settings },
          liveSnapshot(),
          request,
        ).output
      return { output, committed: Promise.resolve(), cacheHit: true }
    }
    const cached = ledger.lookup(
      request.filename,
      request.code,
      request.sourceType,
    )
    if (cached) {
      return {
        output: cached,
        committed: persistence.commitPending(liveSnapshot),
        cacheHit: true,
      }
    }
    const candidate = sealed
      ? extractSealed(
          { live: live.engine, createEngine, configure, settings },
          liveSnapshot(),
          request,
        )
      : undefined
    const output =
      candidate?.output ?? extractRequest(live.engine, settings, request)
    // The engine reports whether this file's CSS or the base sheet changed
    const changed = !sealed && (output.updatedBaseStyle || output.css != null)
    const input =
      prepared ?? createInput(root, request, output.dependencies ?? [])
    if (candidate !== undefined) live.engine = candidate.engine
    ledger.accept(input, output)
    plan.note(request.filename, output.cssFile)
    if (changed) live.revision += 1
    persistence.accept()
    return {
      output,
      committed: persistence.commit(liveSnapshot),
      cacheHit: false,
    }
  }

  return {
    close() {
      replay.close()
      plan.close()
    },
    async extract(request) {
      const startedAt = profileStart()
      try {
        const accepted = await mutate(() => accept(request))
        await accepted.committed
        if (watch) watchListener?.(watchInputs())
        plan.succeed(request.filename)
        reportProfile('coordinator.extract', {
          cacheHit: accepted.cacheHit,
          durationMs: elapsedMs(startedAt),
          filename: request.filename,
          sourceBytes:
            startedAt === undefined
              ? undefined
              : Buffer.byteLength(request.code),
        })
        return toExtractResponse(
          accepted.output,
          settings.singleCss,
          live.config?.files ?? [],
        )
      } catch (error) {
        if (live.generation?.ordinaryInputs === undefined) {
          plan.fail(
            request.filename,
            error instanceof Error ? error.message : String(error),
          )
        }
        throw error
      }
    },
    async css({ fileNum, importMainCss, wait }) {
      if (watch) {
        return mutate(async () => {
          await reconcile()
          return {
            css: live.engine.getCss(fileNum, importMainCss),
            policy: 'dev-current',
          }
        })
      }
      if (!wait) {
        throw new HttpError(
          400,
          locatedError(
            'devup-ui.css',
            'serve a production stylesheet without waiting for it',
            'a production build only gets a complete stylesheet by waiting',
            'request /css with waitForIdle=true.',
          ).message,
        )
      }
      await plan.wait(fileNum)
      return mutate(async () => {
        if (live.generation !== undefined) {
          options.preparedSources?.validateForCssFinalization?.(live.generation)
        }
        sealed = true
        return {
          css: live.engine.getCss(fileNum, importMainCss),
          policy: 'production-complete',
        }
      })
    },
    startup: () => mutate(() => replay.startup()),
    reconcile: (changedPaths) =>
      mutate(() => reconcile(undefined, changedPaths)),
    watchInputs,
    onWatchInputs(listener) {
      watchListener = listener
      return () => {
        if (watchListener === listener) watchListener = undefined
      }
    },
    async flush() {
      await queue
      await persistence.drain()
    },
  }
}
