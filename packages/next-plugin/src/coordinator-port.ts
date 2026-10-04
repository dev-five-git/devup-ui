import { randomUUID } from 'node:crypto'
import {
  existsSync,
  readFileSync,
  renameSync,
  unlinkSync,
  writeFileSync,
} from 'node:fs'
import { isAbsolute, join, resolve } from 'node:path'

/** Logical build identity, shared by endpoint publication and HTTP requests. */
export interface CoordinatorIdentity {
  readonly project: string
  readonly token: string
}

export interface CoordinatorPortInfo extends CoordinatorIdentity {
  readonly version: 1
  readonly port: number
  readonly pid: number
}

export class InvalidCoordinatorPortError extends Error {
  constructor() {
    super(
      'Invalid coordinator descriptor: needs version 1, a valid port/pid, canonical project and UUID token (at most 4096 bytes)',
    )
  }
}

export function formatPortFile(
  port: number,
  pid = process.pid,
  identity: CoordinatorIdentity = {
    project: resolve(process.cwd()),
    token: randomUUID(),
  },
): string {
  const text = JSON.stringify({ version: 1, port, pid, ...identity })
  parsePortFile(text)
  return text
}

export function parsePortFile(text: string): CoordinatorPortInfo {
  if (Buffer.byteLength(text) > 4096) throw new InvalidCoordinatorPortError()
  const data: unknown = JSON.parse(text)
  if (
    typeof data !== 'object' ||
    data === null ||
    !('version' in data) ||
    data.version !== 1 ||
    !('port' in data) ||
    typeof data.port !== 'number' ||
    !Number.isInteger(data.port) ||
    data.port < 1 ||
    data.port > 65535 ||
    !('pid' in data) ||
    typeof data.pid !== 'number' ||
    !Number.isSafeInteger(data.pid) ||
    data.pid <= 0 ||
    !('project' in data) ||
    typeof data.project !== 'string' ||
    !isAbsolute(data.project) ||
    resolve(data.project) !== data.project ||
    !('token' in data) ||
    typeof data.token !== 'string' ||
    !/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(
      data.token,
    )
  )
    throw new InvalidCoordinatorPortError()
  return {
    version: 1,
    port: data.port,
    pid: data.pid,
    project: data.project,
    token: data.token,
  }
}

export function publishPortFile(path: string, info: CoordinatorPortInfo): void {
  const text = formatPortFile(info.port, info.pid, info)
  const temporary = `${path}.${randomUUID()}.tmp`
  try {
    writeFileSync(temporary, text, { encoding: 'utf-8', flag: 'wx' })
    renameSync(temporary, path)
  } finally {
    if (existsSync(temporary)) unlinkSync(temporary)
  }
}

/** Whether a process exists. EPERM means it exists but is not ours. */
export function isProcessAlive(pid: number): boolean {
  try {
    process.kill(pid, 0)
    return true
  } catch (error) {
    return (
      typeof error === 'object' &&
      error !== null &&
      'code' in error &&
      error.code === 'EPERM'
    )
  }
}

export function readOwner(portFile: string): CoordinatorPortInfo | undefined {
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
export function removeOwnPortFile(
  portFile: string,
  identity?: CoordinatorIdentity,
): void {
  const owner = readOwner(portFile)
  if (
    owner?.pid !== process.pid ||
    (identity &&
      (owner.project !== identity.project || owner.token !== identity.token))
  )
    return
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
  if (typeof error !== 'object' || error === null) return false
  if (
    'code' in error &&
    typeof error.code === 'string' &&
    CONNECTION_CODES.has(error.code)
  )
    return true
  // An AggregateError from `localhost` resolution carries its codes inside.
  const inner = 'errors' in error ? error.errors : undefined
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
