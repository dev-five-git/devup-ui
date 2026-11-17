import { existsSync, readFileSync } from 'node:fs'
import { Agent, request } from 'node:http'

import {
  isConnectionError,
  missingPortFileError,
  parsePortFile,
  unreachableCoordinatorError,
} from './coordinator-port'

/**
 * How long a request may go without any response from the coordinator. The
 * coordinator holds `/css` until extraction settles (at most its own
 * `maxWaitMs` per wait, 60s by default), so this is far above any wait it
 * means to make: only a coordinator that accepted the connection and then
 * stopped answering reaches it.
 */
const DEFAULT_RESPONSE_TIMEOUT_MS = 300_000
const PORT_FILE_RETRIES = 20
const PORT_FILE_RETRY_MS = 50

let responseTimeoutMs = DEFAULT_RESPONSE_TIMEOUT_MS
const keepAliveAgent = new Agent({ keepAlive: true })
/** The port each port file named when last read, so each app keeps its own. */
const cachedPorts = new Map<string, number>()

export interface CoordinatorCall {
  path: string
  method: 'GET' | 'POST'
  body?: string
}

export interface CoordinatorReply {
  statusCode: number
  body: string
}

type ReplyCallback = (error: Error | null, reply?: CoordinatorReply) => void

function isValidPort(port: number): boolean {
  return Number.isInteger(port) && port > 0 && port < 65536
}

/**
 * The port `portFile` names, or `undefined` while it names none: missing,
 * or not completely written yet.
 */
function readPort(portFile: string): number | undefined {
  const cached = cachedPorts.get(portFile)
  if (cached !== undefined) return cached
  if (!existsSync(portFile)) return undefined
  const { port } = parsePortFile(readFileSync(portFile, 'utf-8'))
  if (!isValidPort(port)) return undefined
  cachedPorts.set(portFile, port)
  return port
}

function invalidPortFileError(portFile: string): Error {
  return new Error(
    `Coordinator port file ${portFile} names no valid port. ` +
      'Restart the dev server or build so the coordinator writes it again.',
  )
}

function noResponseError(portFile: string, port: number): Error {
  return new Error(
    `Coordinator at 127.0.0.1:${port} (port file ${portFile}) sent no response for ${responseTimeoutMs / 1000}s. ` +
      'It is stuck or another process holds the port: restart the dev server or build.',
  )
}

/** Wait (about one second) for the port file to name a port. */
function waitForPort(
  portFile: string,
  retries: number,
  onPort: (port: number) => void,
  onError: (error: Error) => void,
): void {
  let port: number | undefined
  try {
    port = readPort(portFile)
  } catch (error) {
    onError(error instanceof Error ? error : new Error(String(error)))
    return
  }
  if (port !== undefined) {
    onPort(port)
    return
  }
  if (retries > 0) {
    setTimeout(
      () => waitForPort(portFile, retries - 1, onPort, onError),
      PORT_FILE_RETRY_MS,
    )
    return
  }
  onError(
    existsSync(portFile)
      ? invalidPortFileError(portFile)
      : missingPortFileError(portFile),
  )
}

function send(
  portFile: string,
  port: number,
  call: CoordinatorCall,
  callback: ReplyCallback,
): void {
  let settled = false
  const settle: ReplyCallback = (error, reply) => {
    if (settled) return
    settled = true
    callback(error, reply)
  }
  const req = request(
    {
      hostname: '127.0.0.1',
      port,
      path: call.path,
      method: call.method,
      headers:
        call.body === undefined
          ? undefined
          : { 'Content-Type': 'application/json' },
      agent: keepAliveAgent,
      timeout: responseTimeoutMs,
    },
    (res) => {
      const chunks: Buffer[] = []
      res.on('data', (chunk: Buffer) => chunks.push(chunk))
      res.on('end', () =>
        settle(null, {
          statusCode: res.statusCode ?? 0,
          body: Buffer.concat(chunks).toString('utf-8'),
        }),
      )
    },
  )
  req.on('timeout', () => req.destroy(noResponseError(portFile, port)))
  req.on('error', (error) => settle(error))
  if (call.body !== undefined) req.write(call.body)
  req.end()
}

/**
 * Call the coordinator that `portFile` names.
 *
 * A port that refuses the connection is forgotten and the file read again: a
 * coordinator that restarted publishes a new port, and the call is sent there
 * once. A port that still does not answer, a port file that never names a
 * port, and a coordinator that stops answering mid-request are errors that
 * say which coordinator and how to recover.
 */
export function callCoordinator(
  portFile: string,
  call: CoordinatorCall,
  callback: ReplyCallback,
): void {
  waitForPort(
    portFile,
    PORT_FILE_RETRIES,
    (port) => {
      send(portFile, port, call, (error, reply) => {
        if (error === null || !isConnectionError(error)) {
          callback(error, reply)
          return
        }
        cachedPorts.delete(portFile)
        const unreachable = (cause: Error) => {
          cachedPorts.delete(portFile)
          callback(unreachableCoordinatorError(portFile, cause))
        }
        waitForPort(
          portFile,
          PORT_FILE_RETRIES,
          (next) => {
            if (next === port) {
              unreachable(error)
              return
            }
            send(portFile, next, call, (retryError, retryReply) => {
              if (retryError !== null && isConnectionError(retryError)) {
                unreachable(retryError)
                return
              }
              callback(retryError, retryReply)
            })
          },
          () => unreachable(error),
        )
      })
    },
    (error) => callback(error),
  )
}

/** @internal Forget cached ports and restore the default timeout. */
export function resetCoordinatorClient(): void {
  cachedPorts.clear()
  responseTimeoutMs = DEFAULT_RESPONSE_TIMEOUT_MS
}

/** @internal Shorten the response timeout for tests. */
export function setCoordinatorResponseTimeoutForTesting(ms: number): void {
  responseTimeoutMs = ms
}
