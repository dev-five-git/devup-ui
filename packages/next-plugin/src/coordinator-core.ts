import { resolve } from 'node:path'

import { loadTheme, readConfigState } from './coordinator-config'
import {
  buildEngine,
  type ExtractOutputSnapshot,
  extractRequest,
  type ExtractSettings,
  locatedError,
  toExtractResponse,
} from './coordinator-engine'
import { type ExtractRequest, HttpError } from './coordinator-http'
import {
  createInput,
  createInputLedger,
  isCurrent,
  isGone,
} from './coordinator-ledger'
import type { CoordinatorOptions, Core } from './coordinator-options'
import { createPersistence } from './coordinator-persistence'
import { createProductionPlan } from './coordinator-plan'
import { elapsedMs, profileStart, reportProfile } from './profile'
import {
  type AllocatorState,
  captureCoordinatorState,
  type CoordinatorInput,
  exportAllocatorState,
  readCoordinatorState,
} from './state'
import { createWasm } from './wasm'

interface Accepted {
  output: ExtractOutputSnapshot
  committed: Promise<void>
  cacheHit: boolean
}

interface RebuildRequest {
  survivors: readonly CoordinatorInput[]
  removed: readonly CoordinatorInput[]
  allocator?: AllocatorState
}

/**
 * Everything a coordinator knows, with no HTTP in it: the engine in service,
 * the inputs it holds, completion tracking and persistence. Every change goes
 * through one queue, so the engine is never mutated by two requests at once.
 */
export function createCore(options: CoordinatorOptions, project: string): Core {
  const root = resolve(options.projectRoot ?? process.cwd())
  const watch = options.watch ?? false
  const optionsKey = options.optionsKey ?? ''
  const resolveFile = (file: string | undefined) =>
    file === undefined ? undefined : resolve(root, file)
  const stateFile = resolveFile(options.stateFile)
  const devupFile = resolveFile(options.devupFile)
  const settings: ExtractSettings = {
    package: options.package,
    cssDir: options.cssDir,
    singleCss: options.singleCss,
    sourceMap: options.sourceMap ?? true,
    importAliases: options.importAliases,
  }
  const createEngine = options.createEngine ?? (() => createWasm(root))
  const configure = options.configureWasm ?? (() => undefined)
  const ledger = createInputLedger(options.cacheMaxEntries ?? 4096)
  const plan = createProductionPlan(options)
  const persistence = createPersistence({
    stateFile,
    revisionFile: resolveFile(options.revisionFile),
  })
  const checkpoint =
    watch && stateFile !== undefined
      ? readCoordinatorState(stateFile, optionsKey)
      : undefined

  let engine = options.wasm
  let revision = checkpoint?.revision ?? 0
  let sealed = false
  let config = devupFile === undefined ? undefined : readConfigState(devupFile)

  let queue: Promise<unknown> = Promise.resolve()
  function mutate<T>(task: () => Promise<T>): Promise<T> {
    const run = queue.then(task)
    // The caller sees a failure through `run`; the queue just keeps going.
    queue = run.then(
      () => undefined,
      () => undefined,
    )
    return run
  }

  const liveSnapshot = () =>
    captureCoordinatorState({
      wasm: engine,
      optionsKey,
      project,
      revision,
      inputs: ledger.list(),
    })

  for (const [filename, { source, ...output }] of options.prewarmedOutputs ??
    []) {
    const request = {
      filename,
      code: source,
      resourcePath: resolve(root, filename),
    }
    ledger.accept(createInput(root, request, output.dependencies ?? []), output)
    plan.note(filename, output.cssFile)
  }

  async function rebuild(request: RebuildRequest): Promise<void> {
    const fresh = buildEngine({
      createEngine,
      live: engine,
      configure,
      allocator: request.allocator ?? exportAllocatorState(engine),
      theme: devupFile === undefined ? undefined : loadTheme(devupFile),
      settings,
      inputs: request.survivors,
    })
    revision += 1
    // The fresh engine goes into service only once its state is on disk.
    await persistence.commit(() =>
      captureCoordinatorState({
        wasm: fresh,
        optionsKey,
        project,
        revision,
        inputs: request.survivors,
      }),
    )
    engine = fresh
    ledger.replace(request.survivors)
    for (const input of request.removed) plan.forget(input.filename)
  }

  /** Rebuild when a source was deleted or the theme changed on disk. */
  async function reconcile(superseding?: string): Promise<void> {
    if (!watch) return
    const next =
      devupFile === undefined ? undefined : readConfigState(devupFile)
    const configChanged = next?.signature !== config?.signature
    const removed = ledger.list().filter(isGone)
    if (removed.length === 0 && !configChanged) return
    // A file about to be extracted again is replaced by that extraction.
    const drop = new Set(removed.map((input) => input.filename))
    if (superseding !== undefined) drop.add(superseding)
    await rebuild({
      survivors: ledger.list().filter((input) => !drop.has(input.filename)),
      removed,
    })
    config = next
  }

  async function accept(request: ExtractRequest): Promise<Accepted> {
    await reconcile(request.filename)
    const cached = ledger.lookup(request.filename, request.code)
    if (cached) {
      return {
        output: cached,
        committed: persistence.commitPending(liveSnapshot),
        cacheHit: true,
      }
    }
    const output = extractRequest(engine, settings, request)
    // The engine reports whether this file's CSS or the base sheet changed
    const changed = output.updatedBaseStyle || output.css != null
    if (sealed && changed) {
      throw locatedError(
        request.filename,
        'change styles after the production stylesheet was served',
        'its extraction produced CSS the served stylesheet lacks',
        'include it in expectedBaseFiles (or the prewarm) so its styles exist before CSS is served.',
      )
    }
    ledger.accept(createInput(root, request, output.dependencies ?? []), output)
    plan.note(request.filename, output.cssFile)
    if (changed) revision += 1
    persistence.accept()
    return {
      output,
      committed: persistence.commit(liveSnapshot),
      cacheHit: false,
    }
  }

  return {
    close: () => plan.close(),
    async extract(request) {
      const startedAt = profileStart()
      try {
        const accepted = await mutate(() => accept(request))
        await accepted.committed
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
          config?.files ?? [],
        )
      } catch (error) {
        plan.fail(
          request.filename,
          error instanceof Error ? error.message : String(error),
        )
        throw error
      }
    },
    async css({ fileNum, importMainCss, wait }) {
      if (watch) {
        await mutate(() => reconcile())
        return {
          css: engine.getCss(fileNum, importMainCss),
          policy: 'dev-current',
        }
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
      sealed = true
      return {
        css: engine.getCss(fileNum, importMainCss),
        policy: 'production-complete',
      }
    },
    async startup() {
      if (checkpoint === undefined) {
        await persistence.commit(liveSnapshot)
        return
      }
      await mutate(() =>
        rebuild({
          survivors: checkpoint.inputs.filter(isCurrent),
          removed: [],
          allocator: checkpoint,
        }),
      )
    },
    reconcile: () => mutate(() => reconcile()),
    async flush() {
      await queue
      await persistence.drain()
    },
  }
}
