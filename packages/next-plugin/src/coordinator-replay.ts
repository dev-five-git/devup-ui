import { resolve } from 'node:path'

import { CoordinatorShutdownError } from './coordinator-completion'
import { loadTheme, readConfigState } from './coordinator-config'
import { buildEngine, type ExtractSettings } from './coordinator-engine'
import { immutableGeneration, immutableInput } from './coordinator-generation'
import {
  createInput,
  createInputLedger,
  isCurrent,
  isGone,
  orderInputs,
} from './coordinator-ledger'
import type {
  CoordinatorOptions,
  PreparedSourceGeneration,
} from './coordinator-options'
import { createPersistence } from './coordinator-persistence'
import { createProductionPlan } from './coordinator-plan'
import {
  type AllocatorState,
  captureCoordinatorState,
  type CoordinatorInput,
  exportAllocatorState,
  readCoordinatorState,
} from './state'
import { createWasm } from './wasm'

interface RebuildRequest {
  readonly survivors: readonly CoordinatorInput[]
  readonly removed: readonly CoordinatorInput[]
  readonly allocator?: AllocatorState
  readonly generation?: PreparedSourceGeneration
}

/** Replay state is mutated only by Core's mutation queue. */
export function createReplay(options: CoordinatorOptions, project: string) {
  const root = resolve(options.projectRoot ?? process.cwd())
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
  const fallbackConfigure = options.configureWasm ?? (() => undefined)
  const prepared = options.preparedSources
  const controller = new AbortController()
  const ledger = createInputLedger(options.cacheMaxEntries ?? 4096)
  const plan = createProductionPlan(options)
  const persistence = createPersistence({
    stateFile,
    revisionFile: resolveFile(options.revisionFile),
  })
  const checkpoint =
    prepared === undefined && options.watch && stateFile !== undefined
      ? readCoordinatorState(stateFile, optionsKey)
      : undefined
  const live = {
    engine: options.wasm,
    revision: prepared?.initial.revision ?? checkpoint?.revision ?? 0,
    config: devupFile === undefined ? undefined : readConfigState(devupFile),
    generation:
      prepared === undefined
        ? undefined
        : immutableGeneration(prepared.initial.generation),
  }
  const configure = (wasm: typeof live.engine) =>
    (live.generation?.configureWasm ?? fallbackConfigure)(wasm)
  const snapshot = () =>
    captureCoordinatorState({
      wasm: live.engine,
      optionsKey,
      project,
      revision: live.revision,
      inputs: ledger.list(),
    })

  if (prepared !== undefined) {
    ledger.replace(
      orderInputs([
        ...prepared.initial.ordinaryInputs.map(immutableInput),
        ...prepared.initial.generation.sources.map(({ input }) =>
          immutableInput(input),
        ),
      ]),
    )
  }
  for (const [filename, { source, ...output }] of options.prewarmedOutputs ??
    []) {
    const request = {
      filename,
      code: source,
      resourcePath: resolve(root, filename),
    }
    const input =
      prepared === undefined
        ? createInput(root, request, output.dependencies ?? [])
        : ledger
            .list()
            .find(
              (input) => input.filename === filename && input.source === source,
            )
    if (input === undefined) continue
    ledger.accept(input, output)
    plan.note(filename, output.cssFile)
  }

  async function rebuild(request: RebuildRequest): Promise<void> {
    controller.signal.throwIfAborted()
    const generation = request.generation ?? live.generation
    const fresh = buildEngine({
      createEngine,
      live: live.engine,
      configure: generation?.configureWasm ?? fallbackConfigure,
      allocator: request.allocator ?? exportAllocatorState(live.engine),
      theme: devupFile === undefined ? undefined : loadTheme(devupFile),
      settings,
      inputs: request.survivors,
    })
    const revision = live.revision + 1
    controller.signal.throwIfAborted()
    await persistence.commitCandidate(
      captureCoordinatorState({
        wasm: fresh,
        optionsKey,
        project,
        revision,
        inputs: request.survivors,
      }),
      snapshot(),
    )
    controller.signal.throwIfAborted()
    live.engine = fresh
    live.revision = revision
    live.generation = generation
    ledger.replace(request.survivors)
    for (const input of request.removed) plan.forget(input.filename)
  }

  async function reconcile(superseding?: string): Promise<void> {
    if (!options.watch) return
    controller.signal.throwIfAborted()
    let generation = live.generation
    if (prepared !== undefined && generation !== undefined) {
      const next = await prepared.prepareReplay({
        generation,
        signal: controller.signal,
      })
      controller.signal.throwIfAborted()
      if (next !== generation) generation = immutableGeneration(next)
    }
    const next =
      devupFile === undefined ? undefined : readConfigState(devupFile)
    const configChanged = next?.signature !== live.config?.signature
    const generationChanged = generation !== live.generation
    const owned = new Set(
      live.generation?.sources.map(({ input }) => input.filename),
    )
    const nextOwned = new Set(
      generation?.sources.map(({ input }) => input.filename),
    )
    const inputs = generationChanged
      ? orderInputs([
          ...ledger
            .list()
            .filter(
              (input) =>
                !owned.has(input.filename) && !nextOwned.has(input.filename),
            ),
          ...(generation?.sources.map(({ input }) => input) ?? []),
        ])
      : ledger.list()
    const filenames = new Set(inputs.map((input) => input.filename))
    const removed = [
      ...inputs.filter(isGone),
      ...ledger.list().filter((input) => !filenames.has(input.filename)),
    ]
    if (removed.length === 0 && !configChanged && !generationChanged) return
    const drop = new Set(removed.map((input) => input.filename))
    if (superseding !== undefined && !nextOwned.has(superseding))
      drop.add(superseding)
    await rebuild({
      survivors: inputs.filter((input) => !drop.has(input.filename)),
      removed,
      generation,
    })
    live.config = next
  }

  return {
    root,
    settings,
    createEngine,
    configure,
    ledger,
    plan,
    persistence,
    live,
    snapshot,
    reconcile,
    close: () =>
      controller.abort(new CoordinatorShutdownError('replay preparation')),
    async startup(): Promise<void> {
      controller.signal.throwIfAborted()
      if (checkpoint === undefined) {
        await persistence.commit(snapshot)
        controller.signal.throwIfAborted()
        return
      }
      await rebuild({
        survivors: checkpoint.inputs.filter(isCurrent),
        removed: [],
        allocator: checkpoint,
      })
    },
  }
}
