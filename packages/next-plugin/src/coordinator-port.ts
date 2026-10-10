import { existsSync, readFileSync, unlinkSync } from 'node:fs'
import { join } from 'node:path'

/**
 * What a coordinator writes to its port file: the port on the first line (all
 * an older reader looks at, via `parseInt`) and the owning pid on the second.
 */
export interface CoordinatorPortInfo {
  port: number
  /** Process that owns the coordinator, when the file says. */
  pid?: number
}

export function formatPortFile(port: number, pid = process.pid): string {
  return `${port}\n${pid}`
}

export function parsePortFile(text: string): CoordinatorPortInfo {
  const [portLine = '', pidLine = ''] = text.trim().split(/\r?\n/)
  const port = Number.parseInt(portLine.trim(), 10)
  const pid = Number.parseInt(pidLine.trim(), 10)
  return { port, pid: Number.isInteger(pid) && pid > 0 ? pid : undefined }
}

/** Whether a process exists. EPERM means it exists but is not ours. */
export function isProcessAlive(pid: number): boolean {
  try {
    process.kill(pid, 0)
    return true
  } catch (error) {
    return (error as NodeJS.ErrnoException).code === 'EPERM'
  }
}

function readOwner(portFile: string): CoordinatorPortInfo | undefined {
  try {
    return parsePortFile(readFileSync(portFile, 'utf-8'))
  } catch {
    return undefined
  }
}

/** A live coordinator owned by some other process, if the file names one. */
export function foreignLiveOwner(portFile: string): number | undefined {
  const pid = readOwner(portFile)?.pid
  return pid !== undefined && pid !== process.pid && isProcessAlive(pid)
    ? pid
    : undefined
}

/**
 * The port file this process should use in `distDir`.
 *
 * `next dev` and `next build` in one directory used to share one file: the
 * second one deleted the first one's port and overwrote it, and the first's
 * loaders then talked to a dead port or the wrong coordinator. The shared name
 * is kept when nobody else holds it (so a cache keyed on loader options stays
 * stable in the common case); when a different live process owns it, this
 * process gets its own `coordinator.<pid>.port`.
 */
export function resolveCoordinatorPortFile(distDir: string): string {
  const shared = join(distDir, 'coordinator.port')
  return foreignLiveOwner(shared) === undefined
    ? shared
    : join(distDir, `coordinator.${process.pid}.port`)
}

/** Remove the file unless a different live process owns it. */
export function removeStalePortFile(portFile: string): void {
  if (foreignLiveOwner(portFile) !== undefined) return
  try {
    unlinkSync(portFile)
  } catch {
    // Nothing to remove (first run).
  }
}

/** Remove the file only if this process wrote it. */
export function removeOwnPortFile(portFile: string): void {
  const owner = readOwner(portFile)
  if (owner?.pid !== undefined && owner.pid !== process.pid) return
  try {
    unlinkSync(portFile)
  } catch {
    // Already gone.
  }
}

const CONNECTION_CODES = new Set([
  'ECONNREFUSED',
  'ECONNRESET',
  'ETIMEDOUT',
  'EPIPE',
  'EHOSTUNREACH',
])

export function isConnectionError(error: unknown): boolean {
  const code = (error as NodeJS.ErrnoException | undefined)?.code
  if (code !== undefined && CONNECTION_CODES.has(code)) return true
  // An AggregateError from `localhost` resolution carries its codes inside.
  const inner = (error as { errors?: unknown[] } | undefined)?.errors
  return Array.isArray(inner) && inner.some(isConnectionError)
}

/** The port file is missing: say whose it should be and how to recover. */
export function missingPortFileError(portFile: string): Error {
  return new Error(
    `Coordinator port file not found: ${portFile}. ` +
      'The devup-ui coordinator for this build is not running (or exited). ' +
      'Restart the dev server or build; if another `next dev`/`next build` ' +
      'uses the same distDir, stop it or give this one its own distDir.',
  )
}

/** A coordinator that was running is not answering. */
export function unreachableCoordinatorError(
  portFile: string,
  cause: unknown,
): Error {
  const info = existsSync(portFile) ? readOwner(portFile) : undefined
  const owner =
    info?.pid === undefined
      ? 'unknown owner'
      : `owner pid ${info.pid} (${isProcessAlive(info.pid) ? 'running' : 'not running'})`
  const detail = cause instanceof Error ? cause.message : String(cause)
  const next =
    info?.pid !== undefined && !isProcessAlive(info.pid)
      ? 'The coordinator process has exited: restart the dev server or build.'
      : 'Another process may have replaced the coordinator: restart the dev server or build, and avoid running `next dev` and `next build` in one distDir at the same time.'
  return new Error(
    `Coordinator unreachable at 127.0.0.1:${info?.port ?? '?'} (${owner}, port file ${portFile}): ${detail}. ${next}`,
  )
}
