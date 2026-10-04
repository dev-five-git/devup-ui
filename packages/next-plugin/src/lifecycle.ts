import { rmSync } from 'node:fs'

import type { NextConfig } from 'next'

import type { CoordinatorHandle } from './coordinator-options'
import type { AppSession } from './session'

type Drainers = Map<string, () => Promise<void>>

const SESSIONS = Symbol.for('@devup-ui/next-plugin/sessions')

/**
 * Next evaluates the config more than once in a process, each time in a fresh
 * module instance. The sessions that own a coordinator are therefore kept where
 * every instance finds them: the later evaluation can drain the coordinator the
 * first one started without being able to close a different app's.
 */
function sessions(): Drainers {
  const holder: typeof globalThis & { [SESSIONS]?: Drainers } = globalThis
  return (holder[SESSIONS] ??= new Map())
}

function describe(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause)
}

interface RetainFields {
  session: AppSession
  coordinator: CoordinatorHandle
}

/**
 * Tie the coordinator's lifetime to the process.
 *
 * `beforeExit` (the event loop is empty, async work is still possible) awaits
 * the drain, so every accepted write is on disk, and then closes. `exit` cannot
 * wait for anything: it only closes synchronously, as a last resort.
 */
export function retainSession({ session, coordinator }: RetainFields): void {
  let closed = false
  const closeOnce = (): void => {
    if (closed) return
    closed = true
    sessions().delete(session.token)
    coordinator.close()
    rmSync(session.sessionDir, { recursive: true, force: true })
  }
  const drainAndClose = async (): Promise<void> => {
    try {
      await coordinator.drain()
    } catch (cause) {
      console.error(
        `${session.sessionDir}:1:1: devup-ui cannot use \`the final state write\` at build time: ${describe(cause)}; needs a writable distDir.`,
      )
    } finally {
      closeOnce()
    }
  }

  sessions().set(session.token, () => coordinator.drain())
  coordinator.ready.catch((cause: unknown) => {
    console.error(
      `${session.endpointFile}:1:1: devup-ui coordinator cannot use \`a loopback listener\` at build time: ${describe(cause)}; needs permission to listen on 127.0.0.1 and to write its endpoint file.`,
    )
  })
  // Once: the drain schedules async work, which makes the loop run, and empty
  // again, and Node would emit beforeExit a second time.
  process.once('beforeExit', drainAndClose)
  process.once('exit', closeOnce)
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
  const previous = config.compiler.runAfterProductionCompile
  config.compiler.runAfterProductionCompile = async (metadata) => {
    await sessions().get(token)?.()
    await previous?.(metadata)
  }
}
