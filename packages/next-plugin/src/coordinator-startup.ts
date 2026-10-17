import { isCurrent } from './coordinator-ledger'
import type {
  AllocatorState,
  CoordinatorInput,
  readCoordinatorState,
} from './state'

interface StartupReplay {
  readonly survivors: readonly CoordinatorInput[]
  readonly removed: readonly CoordinatorInput[]
  readonly allocator: AllocatorState
}

export function createReplayStartup(
  checkpoint: ReturnType<typeof readCoordinatorState>,
  signal: AbortSignal,
  owner: {
    readonly commit: () => Promise<void>
    readonly rebuild: (request: StartupReplay) => Promise<void>
  },
): () => Promise<void> {
  return async () => {
    signal.throwIfAborted()
    if (checkpoint === undefined) {
      await owner.commit()
      signal.throwIfAborted()
      return
    }
    await owner.rebuild({
      survivors: checkpoint.inputs.filter(isCurrent),
      removed: [],
      allocator: checkpoint,
    })
  }
}
