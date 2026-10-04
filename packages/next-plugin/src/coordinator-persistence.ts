import { writeFileAtomically } from '@devup-ui/plugin-utils'

import {
  type CoordinatorSnapshot,
  CoordinatorStateError,
  createSnapshotCommitter,
  writeCoordinatorState,
} from './state'

export interface Persistence {
  /** Note that a new state was accepted and is not on disk yet. */
  accept(): void
  /** Commit the state `capture` returns, covering everything accepted so far. */
  commit(capture: () => CoordinatorSnapshot): Promise<void>
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
  const persistence: Persistence = {
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
    commitPending(capture) {
      return accepted > persisted
        ? persistence.commit(capture)
        : Promise.resolve()
    },
    drain: () => committer.drain(),
  }
  return persistence
}
