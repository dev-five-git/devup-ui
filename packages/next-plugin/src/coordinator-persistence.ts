import { writeFileAtomically } from '@devup-ui/plugin-utils'

import {
  type CommitControl,
  commitTransaction,
  CompensationError,
} from './coordinator-transaction'
import {
  type CoordinatorSnapshot,
  CoordinatorStateError,
  createSnapshotCommitter,
  writeCoordinatorState,
} from './state'

export interface Persistence {
  assertHealthy(): void
  /** Note that a new state was accepted and is not on disk yet. */
  accept(): void
  /** Commit the state `capture` returns, covering everything accepted so far. */
  commit(capture: () => CoordinatorSnapshot): Promise<void>
  /** A replay candidate is isolated until durable; restore its predecessor if revision publication fails. */
  commitCandidate(
    snapshot: CoordinatorSnapshot,
    previous: CoordinatorSnapshot,
    control?: CommitControl,
  ): Promise<void>
  /**
   * Commit the same way, but only if something accepted is not on disk yet:
   * a repeated source is acknowledged once what came before it is durable,
   * including after a write that failed.
   */
  commitPending(capture: () => CoordinatorSnapshot): Promise<void>
  /** Rejects if the latest complete capture could not be committed. */
  drain(): Promise<void>
}

/**
 * Where accepted state goes: one checkpoint file, plus a revision file that
 * changes only when the revision does (loaders watch it for new CSS).
 * Without either file nothing is persisted and nothing is waited for.
 */
export function createPersistence(files: {
  readonly stateFile?: string
  readonly revisionFile?: string
}): Persistence {
  const { stateFile, revisionFile } = files
  let writtenRevision: number | undefined
  const predecessors = new WeakMap<CoordinatorSnapshot, CoordinatorSnapshot>()
  const committer = createSnapshotCommitter(async (snapshot) => {
    if (stateFile !== undefined) {
      try {
        await writeCoordinatorState(stateFile, snapshot)
      } catch (cause) {
        throw new CoordinatorStateError(
          stateFile,
          'could not be committed; it needs a writable checkpoint destination',
          cause,
        )
      }
    }
    if (revisionFile !== undefined && snapshot.revision !== writtenRevision) {
      try {
        await writeFileAtomically(revisionFile, String(snapshot.revision))
      } catch (cause) {
        const previous = predecessors.get(snapshot)
        if (stateFile !== undefined && previous !== undefined) {
          await writeCoordinatorState(stateFile, previous)
        }
        throw new CoordinatorStateError(
          revisionFile,
          'could not publish the committed revision; it needs a writable revision destination',
          cause,
        )
      }
      writtenRevision = snapshot.revision
    }
  })
  const enabled = stateFile !== undefined || revisionFile !== undefined
  let accepted = 0
  let persisted = 0
  let quarantine: CompensationError | undefined
  const persistence: Persistence = {
    assertHealthy() {
      if (quarantine !== undefined) throw quarantine
    },
    accept() {
      accepted += 1
    },
    commit(capture) {
      const covers = accepted
      return enabled
        ? committer.commit(capture).then(() => {
            persisted = Math.max(persisted, covers)
          })
        : Promise.resolve()
    },
    async commitCandidate(snapshot, previous, control) {
      if (quarantine !== undefined) throw quarantine
      if (control !== undefined) {
        await committer.drain()
        try {
          await commitTransaction(files, snapshot, {
            signal: control.signal,
            publish() {
              control.publish()
              writtenRevision = snapshot.revision
              persisted = accepted
            },
          })
        } catch (error) {
          if (error instanceof CompensationError) quarantine = error
          throw error
        }
        return
      }
      predecessors.set(snapshot, previous)
      try {
        await persistence.commit(() => snapshot)
      } finally {
        predecessors.delete(snapshot)
      }
    },
    commitPending(capture) {
      return accepted > persisted
        ? persistence.commit(capture)
        : Promise.resolve()
    },
    async drain() {
      persistence.assertHealthy()
      await committer.drain()
    },
  }
  return persistence
}
