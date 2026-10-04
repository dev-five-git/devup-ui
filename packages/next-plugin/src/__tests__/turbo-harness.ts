import { afterEach, beforeEach, mock, spyOn } from 'bun:test'

import * as coordinatorModule from '../coordinator'
import type {
  CoordinatorHandle,
  CoordinatorOptions,
} from '../coordinator-options'
import { resetTurboSetupCacheForTesting } from '../plugin'

type Handler = () => unknown

export interface TurboHarness {
  /** The options of every coordinator the plugin started */
  readonly starts: CoordinatorOptions[]
  /** The handle returned for each of them */
  readonly handles: CoordinatorHandle[]
  /** The process-lifetime handlers the plugin registered, by event */
  readonly handlers: Record<string, Handler[]>
  /** Runs when a coordinator is started, before it publishes anything */
  onStart: ((options: CoordinatorOptions) => void) | undefined
  /** Start the coordinators for real instead of recording them */
  startRealCoordinators(): void
}

/**
 * Run the plugin as Next does: Turbopack on, the coordinator recorded instead
 * of started, and process-lifetime handlers captured instead of registered.
 */
export function installTurboHarness(): TurboHarness {
  const spies: {
    start?: ReturnType<typeof spyOn>
    once?: ReturnType<typeof spyOn>
  } = {}
  const harness: TurboHarness = {
    starts: [],
    handles: [],
    handlers: {},
    onStart: undefined,
    startRealCoordinators: () => spies.start?.mockRestore(),
  }
  let originalEnv: NodeJS.ProcessEnv

  beforeEach(() => {
    resetTurboSetupCacheForTesting()
    originalEnv = { ...process.env }
    process.env.TURBOPACK = '1'
    process.env.NODE_ENV = 'production'
    harness.starts.length = 0
    harness.handles.length = 0
    harness.onStart = undefined
    for (const event of Object.keys(harness.handlers)) {
      delete harness.handlers[event]
    }
    spies.start = spyOn(
      coordinatorModule,
      'startCoordinator',
    ).mockImplementation((options) => {
      harness.onStart?.(options)
      const handle: CoordinatorHandle = {
        ready: Promise.resolve(),
        close: mock(() => {}),
        drain: mock(async () => {}),
      }
      harness.starts.push(options)
      harness.handles.push(handle)
      return handle
    })
    spies.once = spyOn(process, 'once').mockImplementation(((
      event: string,
      handler: Handler,
    ) => {
      ;(harness.handlers[event] ??= []).push(handler)
      return process
    }) as never)
  })
  afterEach(() => {
    spies.start?.mockRestore()
    spies.once?.mockRestore()
    resetTurboSetupCacheForTesting()
    process.env = originalEnv
  })
  return harness
}
