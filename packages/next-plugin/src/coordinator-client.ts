import { readFileSync } from 'node:fs'
import { request } from 'node:http'
import { setTimeout as delay } from 'node:timers/promises'

import type {
  CoordinatorIdentity,
  CoordinatorPortInfo,
} from './coordinator-port'
import { isConnectionError, parsePortFile } from './coordinator-port'

export interface CoordinatorRequest {
  readonly portFile: string
  readonly identity?: CoordinatorIdentity
  readonly resourcePath: string
  readonly path: string
  readonly method?: 'GET' | 'POST'
  readonly body?: string
  readonly timeoutMs?: number
}

export class CoordinatorRequestError extends Error {
  constructor(
    readonly operation: CoordinatorRequest,
    cause: unknown,
  ) {
    super(
      `${operation.resourcePath}:1:1: devup-ui coordinator cannot use \`${operation.path.split('?')[0]}\` at build time: ${cause instanceof Error ? cause.message : String(cause)}; port file ${operation.portFile}. It needs a live coordinator with matching project/token ownership; restart the dev server or build.`,
    )
  }
}

class TransportFailure extends Error {
  constructor(
    readonly cause: Error,
    readonly retryable: boolean,
  ) {
    super(cause.message)
  }
}

function exchange(options: {
  readonly endpoint: CoordinatorPortInfo
  readonly path: string
  readonly method: 'GET' | 'POST'
  readonly body?: string
  readonly deadline: number
}): Promise<string> {
  if (performance.now() >= options.deadline)
    return Promise.reject(new Error('Coordinator request deadline exceeded'))
  return new Promise((resolve, reject) => {
    let settled = false
    let connected = false
    const finish = (error?: Error, body?: string) => {
      if (settled) return
      settled = true
      clearTimeout(timer)
      if (error) reject(error)
      else resolve(body ?? '')
    }
    const req = request(
      {
        hostname: '127.0.0.1',
        port: options.endpoint.port,
        path: options.path,
        method: options.method,
        agent: false,
        headers: {
          'Content-Type': 'application/json',
          'x-devup-project': encodeURIComponent(options.endpoint.project),
          'x-devup-token': options.endpoint.token,
        },
      },
      (res) => {
        const chunks: Buffer[] = []
        let bytes = 0
        res.on('data', (chunk: Buffer) => {
          bytes += chunk.length
          if (options.path === '/health' && bytes > 4096) {
            finish(new Error('Health descriptor exceeds 4096 bytes'))
            res.destroy()
            req.destroy()
            return
          }
          chunks.push(chunk)
        })
        const lost = (error: Error) =>
          finish(
            new TransportFailure(
              error,
              options.method === 'GET' && isConnectionError(error),
            ),
          )
        res.on('error', lost)
        res.on('aborted', () =>
          lost(
            Object.assign(new Error('Coordinator response aborted'), {
              code: 'ECONNRESET',
            }),
          ),
        )
        res.on('end', () => {
          const content = Buffer.concat(chunks).toString('utf-8')
          if (res.statusCode !== 200)
            finish(new Error(`HTTP ${res.statusCode}: ${content}`))
          else finish(undefined, content)
        })
      },
    )
    const timer = setTimeout(
      () => {
        finish(new Error('Coordinator request deadline exceeded'))
        req.destroy()
      },
      Math.max(1, options.deadline - performance.now()),
    )
    req.on('socket', (socket) =>
      socket.once('connect', () => {
        connected = true
      }),
    )
    req.on('error', (error) =>
      finish(
        new TransportFailure(
          error,
          isConnectionError(error) && (options.method === 'GET' || !connected),
        ),
      ),
    )
    req.end(options.body)
  })
}

export async function requestCoordinator(
  options: CoordinatorRequest,
): Promise<string> {
  const deadline =
    performance.now() +
    (options.timeoutMs ?? (options.method === 'POST' ? 60000 : 75000))
  let reason: unknown = new Error(
    'Coordinator endpoint missing or deadline exceeded',
  )
  let identity = options.identity
  while (performance.now() < deadline) {
    try {
      const endpoint = parsePortFile(readFileSync(options.portFile, 'utf-8'))
      identity ??= endpoint
      if (
        endpoint.project !== identity.project ||
        endpoint.token !== identity.token
      )
        throw new Error('Coordinator endpoint identity mismatch')
      const health = parsePortFile(
        await exchange({ endpoint, path: '/health', method: 'GET', deadline }),
      )
      if (
        health.project !== endpoint.project ||
        health.token !== endpoint.token ||
        health.pid !== endpoint.pid ||
        health.port !== endpoint.port
      )
        throw new Error('Coordinator health ownership mismatch')
      return await exchange({
        endpoint,
        path: options.path,
        method: options.method ?? 'GET',
        body: options.body,
        deadline,
      })
    } catch (error) {
      reason = error
      const missing =
        error instanceof Error && 'code' in error && error.code === 'ENOENT'
      if (!missing && !(error instanceof TransportFailure && error.retryable))
        throw new CoordinatorRequestError(options, error)
      await delay(Math.max(0, Math.min(20, deadline - performance.now())))
    }
  }
  throw new CoordinatorRequestError(options, reason)
}
