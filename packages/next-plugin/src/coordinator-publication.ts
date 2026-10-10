import type { readConfigState } from './coordinator-config'
import type { ExtractOutputSnapshot } from './coordinator-engine'
import type { InputLedger } from './coordinator-ledger'
import type { PreparedSourceGeneration } from './coordinator-options'
import type { ProductionPlan } from './coordinator-plan'
import { CompensationError } from './coordinator-transaction'
import type { CoordinatorInput } from './state'
import type { DevupWasm } from './wasm'

interface LiveState {
  engine: DevupWasm
  revision: number
  generation: PreparedSourceGeneration | undefined
  config: ReturnType<typeof readConfigState> | undefined
}

interface PublicationOwner {
  readonly live: LiveState
  readonly ledger: InputLedger
  readonly plan: ProductionPlan
  readonly signal: AbortSignal
  readonly onWatchInputs?: (inputs: readonly string[]) => void
}

interface PublicationCandidate {
  readonly engine: DevupWasm
  readonly revision: number
  readonly generation: PreparedSourceGeneration | undefined
  readonly config: LiveState['config']
  readonly survivors: readonly CoordinatorInput[]
  readonly removed: readonly CoordinatorInput[]
  readonly outputs: ReadonlyMap<string, ExtractOutputSnapshot>
  readonly transactional: boolean
}

export function stagePublication(
  owner: PublicationOwner,
  candidate: PublicationCandidate,
): () => void {
  const { live, ledger, plan, signal, onWatchInputs } = owner
  const publishLedger = ledger.stage(
    candidate.survivors,
    candidate.generation?.ordinaryInputs !== undefined
      ? candidate.outputs
      : new Map(),
    candidate.outputs,
  )
  const publishPlan =
    candidate.generation?.plan === undefined
      ? undefined
      : plan.stage(candidate.generation.plan, candidate.outputs)
  const previousWatches = [
    ...(live.generation?.watchInputs ?? []),
    ...ledger.resolutionInputs().map((input) => input.path),
  ]
  const nextWatches = [
    ...(candidate.generation?.watchInputs ?? []),
    ...[...candidate.outputs.values()].flatMap((output) =>
      (output.resolutionInputs ?? []).map((input) => input.path),
    ),
  ]
  return () => {
    if (candidate.transactional && onWatchInputs !== undefined) {
      try {
        onWatchInputs(nextWatches)
        signal.throwIfAborted()
      } catch (cause) {
        try {
          onWatchInputs(previousWatches)
        } catch (compensation) {
          throw new CompensationError(
            'replay publication',
            'failed watch compensation; this owner must restart',
            new AggregateError([cause, compensation]),
          )
        }
        throw cause
      }
    }
    signal.throwIfAborted()
    live.engine = candidate.engine
    live.revision = candidate.revision
    live.generation = candidate.generation
    publishLedger()
    publishPlan?.()
    for (const input of candidate.removed) plan.forget(input.filename)
    live.config = candidate.config ?? live.config
    if (!candidate.transactional) onWatchInputs?.(nextWatches)
  }
}
