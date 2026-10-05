import { resolve } from 'node:path'

import { CoordinatorShutdownError } from './coordinator-completion'
import { loadTheme, readConfigState } from './coordinator-config'
import {
  buildEngine,
  type ExtractOutputSnapshot,
  type ExtractSettings,
} from './coordinator-engine'
import {
  assertAdoptedRequest,
  immutableGeneration,
  immutableInput,
  overlayGeneration,
} from './coordinator-generation'
import type { ExtractRequest } from './coordinator-http'
import {
  createInputLedger,
  isCurrent,
  isGone,
  orderInputs,
  seedPrewarmedOutputs,
} from './coordinator-ledger'
import { prepareObservedReplay } from './coordinator-observation'
import type {
  CoordinatorOptions,
  PreparedSourceGeneration,
} from './coordinator-options'
import { createPersistence } from './coordinator-persistence'
import { createProductionPlan } from './coordinator-plan'
import { stagePublication } from './coordinator-publication'
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
  readonly config?: ReturnType<typeof readConfigState>
}

/** Replay state is mutated only by Core's mutation queue. */
export function createReplay(
  options: CoordinatorOptions,
  project: string,
  onWatchInputs?: (inputs: readonly string[]) => void,
) {
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
  const plan = createProductionPlan(options, prepared?.initial.generation.plan)
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
      ...(live.generation?.plan === undefined
        ? {}
        : { plan: live.generation.plan }),
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
  seedPrewarmedOutputs(ledger, options, plan)

  async function rebuild(request: RebuildRequest): Promise<void> {
    controller.signal.throwIfAborted()
    const generation = request.generation ?? live.generation
    const transactional = generation?.ordinaryInputs !== undefined
    if (transactional) await persistence.drain()
    const outputs = new Map<string, ExtractOutputSnapshot>()
    const fresh = buildEngine({
      createEngine,
      live: live.engine,
      configure: generation?.configureWasm ?? fallbackConfigure,
      allocator: request.allocator ?? exportAllocatorState(live.engine),
      theme: devupFile === undefined ? undefined : loadTheme(devupFile),
      settings,
      inputs: request.survivors,
      outputs,
    })
    const revision = live.revision + 1
    const publish = stagePublication(
      { live, ledger, plan, signal: controller.signal, onWatchInputs },
      {
        engine: fresh,
        revision,
        generation,
        config: request.config,
        survivors: request.survivors,
        removed: request.removed,
        outputs,
        transactional,
      },
    )
    controller.signal.throwIfAborted()
    await persistence.commitCandidate(
      captureCoordinatorState({
        wasm: fresh,
        optionsKey,
        project,
        revision,
        inputs: request.survivors,
        ...(generation?.plan === undefined ? {} : { plan: generation.plan }),
      }),
      snapshot(),
      transactional ? { signal: controller.signal, publish } : undefined,
    )
    if (!transactional) publish()
  }

  async function reconcile(
    superseding?: ExtractRequest,
    changedPaths?: readonly string[],
  ): Promise<void> {
    if (!options.watch) return
    controller.signal.throwIfAborted()
    let generation = live.generation
    if (prepared !== undefined && generation !== undefined) {
      const next = await prepareObservedReplay(
        prepared,
        {
          generation,
          signal: controller.signal,
          ...(changedPaths === undefined
            ? {}
            : { changedPaths: Object.freeze([...changedPaths]) }),
        },
        {
          createEngine,
          live: live.engine,
          allocator: exportAllocatorState(live.engine),
          theme: devupFile === undefined ? undefined : loadTheme(devupFile),
          settings,
        },
      )
      controller.signal.throwIfAborted()
      if (next !== generation) generation = immutableGeneration(next)
    }
    const next =
      devupFile === undefined ? undefined : readConfigState(devupFile)
    const configChanged = next?.signature !== live.config?.signature
    const generationChanged = generation !== live.generation
    const nextOwned = new Set(
      generation?.sources.map(({ input }) => input.filename),
    )
    const inputs = generationChanged
      ? orderInputs(
          generation === undefined
            ? ledger.list()
            : overlayGeneration(ledger.list(), live.generation, generation),
        )
      : ledger.list()
    if (superseding !== undefined && generation?.ordinaryInputs !== undefined)
      assertAdoptedRequest(inputs, superseding)
    const filenames = new Set(inputs.map((input) => input.filename))
    const removed = [
      ...inputs.filter(isGone),
      ...ledger.list().filter((input) => !filenames.has(input.filename)),
    ]
    if (removed.length === 0 && !configChanged && !generationChanged) return
    const drop = new Set(removed.map((input) => input.filename))
    if (
      superseding !== undefined &&
      !nextOwned.has(superseding.filename) &&
      generation?.ordinaryInputs === undefined
    )
      drop.add(superseding.filename)
    await rebuild({
      survivors: inputs.filter((input) => !drop.has(input.filename)),
      removed,
      generation,
      config: next,
    })
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
