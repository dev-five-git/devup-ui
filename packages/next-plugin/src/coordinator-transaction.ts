import { readFile, rm } from 'node:fs/promises'

import { writeFileAtomically } from '@devup-ui/plugin-utils'

import { type CoordinatorSnapshot, CoordinatorStateError } from './state'

export interface TransactionFiles {
  readonly stateFile?: string
  readonly revisionFile?: string
}

export interface CommitControl {
  readonly signal: AbortSignal
  readonly publish: () => void
}

async function readPrevious(path: string): Promise<string | undefined> {
  try {
    return await readFile(path, 'utf8')
  } catch (error) {
    if (error instanceof Error && 'code' in error && error.code === 'ENOENT')
      return undefined
    throw error
  }
}

export class CompensationError extends CoordinatorStateError {
  constructor(stateFile: string, reason: string, cause?: unknown) {
    super(stateFile, reason, cause)
  }
}

export async function commitTransaction(
  files: TransactionFiles,
  snapshot: CoordinatorSnapshot,
  control: CommitControl,
): Promise<void> {
  const { signal } = control
  signal.throwIfAborted()
  const before = new Map<string, string | undefined>()
  for (const path of [files.stateFile, files.revisionFile]) {
    if (path !== undefined) before.set(path, await readPrevious(path))
  }
  signal.throwIfAborted()
  const written: string[] = []
  try {
    for (const [path, value] of [
      [files.stateFile, JSON.stringify(snapshot)],
      [files.revisionFile, String(snapshot.revision)],
    ] as const) {
      if (path === undefined) continue
      await writeFileAtomically(path, value)
      written.push(path)
      signal.throwIfAborted()
    }
    signal.throwIfAborted()
    control.publish()
  } catch (cause) {
    try {
      for (const path of written.reverse()) {
        const previous = before.get(path)
        if (previous === undefined) await rm(path, { force: true })
        else await writeFileAtomically(path, previous)
      }
    } catch (compensation) {
      throw new CompensationError(
        files.stateFile ?? files.revisionFile ?? 'devup-ui.css',
        'failed predecessor compensation; this owner is quarantined and must restart',
        new AggregateError([cause, compensation]),
      )
    }
    throw cause
  }
}
