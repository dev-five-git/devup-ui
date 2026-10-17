import { resolve } from 'node:path'

import {
  type CoordinatorInstance,
  createInstance,
} from './coordinator-instance'
import type {
  CoordinatorStartOptions,
  PreparedCoordinatorHandle,
} from './coordinator-options'

export { takeExtractOutput } from './coordinator-engine'
export type {
  CoordinatorHandle,
  CoordinatorOptions,
  CoordinatorStartOptions,
  DeferredCoordinatorOptions,
  PreparedCoordinatorHandle,
  PrewarmedOutput,
} from './coordinator-options'

interface Entry {
  readonly instance: CoordinatorInstance
  refs: number
}

// The only module-level state: which endpoints have a live coordinator, so a
// config evaluated twice for the same app shares one and an unrelated app is
// never touched. Extraction state lives in each instance.
const registry = new Map<string, Entry>()

function entryFor(key: string, options: CoordinatorStartOptions): Entry {
  const existing = registry.get(key)
  if (existing === undefined) {
    const entry = { instance: createInstance(options), refs: 0 }
    registry.set(key, entry)
    return entry
  }
  const { project, token } = existing.instance.identity
  if (
    options.identity?.project !== project ||
    options.identity.token !== token
  ) {
    throw new Error(
      `${key}:1:1: devup-ui coordinator cannot start: another coordinator (project ${project}) already owns this endpoint. Fix: give each app its own distDir/port file, or pass the same identity to share one coordinator.`,
    )
  }
  return existing
}

/**
 * Start (or join) the coordinator for `options.coordinatorPortFile`. Handles
 * for the same endpoint and identity share one instance, which stops when the
 * last handle is released.
 */
export function startCoordinator(
  options: CoordinatorStartOptions,
): PreparedCoordinatorHandle {
  const key = resolve(
    options.projectRoot ?? process.cwd(),
    options.coordinatorPortFile,
  )
  const entry = entryFor(key, options)
  entry.refs += 1
  let released = false
  /** Whether this handle held the last reference. */
  function release(): boolean {
    if (released) return false
    released = true
    entry.refs -= 1
    if (entry.refs > 0) return false
    if (registry.get(key) === entry) registry.delete(key)
    return true
  }
  return {
    ready: entry.instance.ready,
    prepared: entry.instance.prepared,
    close() {
      if (release()) entry.instance.close()
    },
    async drain() {
      if (release()) await entry.instance.drain()
    },
  }
}

/** @internal Wait for every live coordinator's accepted writes to land. */
export async function flushCoordinatorWrites(): Promise<void> {
  await Promise.all(
    [...registry.values()].map(({ instance }) => instance.flush()),
  )
}

/** @internal Stop every live coordinator (tests). */
export function resetCoordinator(): void {
  for (const { instance } of registry.values()) instance.close()
  registry.clear()
}
