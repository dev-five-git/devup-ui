import { rmSync } from 'node:fs'

import type { NextConfig } from 'next'

import type { CoordinatorHandle } from './coordinator-options'
import type { AppContext, AppSession } from './session'
import type { SetupHandoff } from './setup-handoff'
import type { DevupWasm } from './wasm'

export interface LiveSetup {
  readonly context: AppContext
  readonly engine: DevupWasm
  readonly result: Omit<SetupHandoff, 'key'>
}

export interface SessionOwner {
  readonly session: AppSession
  readonly coordinator: CoordinatorHandle
  readonly setup?: LiveSetup
  readonly drain: () => Promise<void>
  readonly afterCompile: () => Promise<void>
  readonly close: () => void
}

// A released token stays reserved: an old exit callback still owns its directory.
type Owners = Map<string, SessionOwner | undefined>

const SESSIONS = Symbol.for('@devup-ui/next-plugin/sessions')

/**
 * Next evaluates the config more than once in a process, each time in a fresh
 * module instance. The sessions that own a coordinator are therefore kept where
 * every instance finds them: the later evaluation can drain the coordinator the
 * first one started without being able to close a different app's.
 */
function sessions(): Owners {
  const holder: typeof globalThis & { [SESSIONS]?: Owners } = globalThis
  return (holder[SESSIONS] ??= new Map())
}

export function findSessionOwner(token: string): SessionOwner | undefined {
  return sessions().get(token)
}

export class SessionOwnershipError extends Error {
  readonly name = 'SessionOwnershipError'
  constructor(readonly endpointFile: string) {
    super(
      `${endpointFile}:1:1: devup-ui cannot use \`session ownership\` at build time: this session token already has an owner`,
    )
  }
}

function describe(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause)
}

interface RetainFields {
  readonly session: AppSession
  readonly coordinator: CoordinatorHandle
  readonly setup?: LiveSetup
  readonly releaseAdapter?: () => void
}

/**
 * Tie the coordinator's lifetime to the process.
 *
 * `beforeExit` (the event loop is empty, async work is still possible) awaits
 * the drain, so every accepted write is on disk, and then closes. `exit` cannot
 * wait for anything: it only closes synchronously, as a last resort.
 */
export function retainSession({
  session,
  coordinator,
  setup,
  releaseAdapter,
}: RetainFields): SessionOwner {
  if (sessions().has(session.token)) {
    throw new SessionOwnershipError(session.endpointFile)
  }
  let closed = false
  let released = false
  const release = (): void => {
    if (released) return
    released = true
    sessions().set(session.token, undefined)
    releaseAdapter?.()
  }
  const closeOnce = (): void => {
    if (closed) return
    closed = true
    release()
    coordinator.close()
    rmSync(session.sessionDir, { recursive: true, force: true })
  }
  const drain = (): Promise<void> => {
    release()
    return coordinator.drain()
  }
  const drainAndClose = async (): Promise<void> => {
    try {
      await drain()
    } catch (cause) {
      console.error(
        `${session.sessionDir}:1:1: devup-ui cannot use \`the final state write\` at build time: ${describe(cause)}; needs a writable distDir.`,
      )
    } finally {
      closeOnce()
    }
  }

  const owner: SessionOwner = Object.freeze({
    session,
    coordinator,
    ...(setup === undefined ? {} : { setup }),
    drain,
    afterCompile: () => (closed ? Promise.resolve() : drain()),
    close: closeOnce,
  })
  sessions().set(session.token, owner)
  coordinator.ready.catch((cause: unknown) => {
    console.error(
      `${session.endpointFile}:1:1: devup-ui coordinator cannot use \`a loopback listener\` at build time: ${describe(cause)}; needs permission to listen on 127.0.0.1 and to write its endpoint file.`,
    )
  })
  // Once: the drain schedules async work, which makes the loop run, and empty
  // again, and Node would emit beforeExit a second time.
  process.once('beforeExit', drainAndClose)
  process.once('exit', closeOnce)
  return owner
}

/**
 * Drain the coordinator in `runAfterProductionCompile`: every source loader has
 * finished by then, so no request is cut short, and the accepted work is
 * written before the type-check and static generation that follow. The user's
 * own hook runs after the drain, with its result unchanged.
 */
export function installAfterCompileDrain(
  config: NextConfig,
  token: string,
): void {
  config.compiler ??= {}
  const afterCompile = findSessionOwner(token)?.afterCompile
  const previous = config.compiler.runAfterProductionCompile
  config.compiler.runAfterProductionCompile = async (metadata) => {
    await afterCompile?.()
    await previous?.(metadata)
  }
}
