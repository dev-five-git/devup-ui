import { randomUUID } from 'node:crypto'
import {
  createServer,
  type IncomingMessage,
  type ServerResponse,
} from 'node:http'
import type { Socket } from 'node:net'
import { resolve } from 'node:path'

import {
  CoordinatorShutdownError,
  IncompleteCssError,
} from './coordinator-completion'
import {
  assertOwnership,
  HttpError,
  parseExtractRequest,
  parseFileNum,
  readBody,
  sendJson,
} from './coordinator-http'
import type { CoordinatorStartOptions } from './coordinator-options'
import {
  type CoordinatorIdentity,
  formatPortFile,
  publishPortFile,
  removeOwnPortFile,
} from './coordinator-port'
import {
  createPreparation,
  reportBackgroundError,
} from './coordinator-preparation'

export interface CoordinatorInstance {
  readonly identity: CoordinatorIdentity
  readonly ready: Promise<void>
  readonly prepared: Promise<void>
  /** Stop listening and watching now. Accepted writes may still be running. */
  close(): void
  /** Refuse new requests, wait for accepted work and its writes, then close. */
  drain(): Promise<void>
  /** Wait for accepted work and its writes without closing. */
  flush(): Promise<void>
}

/** One app's coordinator: an HTTP endpoint over its own core and engine. */
export function createInstance(
  options: CoordinatorStartOptions,
): CoordinatorInstance {
  const root = resolve(options.projectRoot ?? process.cwd())
  const portFile = resolve(root, options.coordinatorPortFile)
  const identity = options.identity ?? { project: root, token: randomUUID() }
  const enforce = options.identity !== undefined
  const preparation = createPreparation(options, identity)
  const prepared = preparation.wait().then(() => undefined)
  void prepared.then(undefined, reportBackgroundError)
  const inflight = new Set<Promise<void>>()
  const sockets = new Set<Socket>()
  const reading = new Set<IncomingMessage>()
  let draining = false
  let closed = false
  let port = 0

  async function route(
    req: IncomingMessage,
    res: ServerResponse,
    url: URL,
  ): Promise<void> {
    if (req.method === 'GET' && url.pathname === '/health') {
      res.writeHead(200, { 'Content-Type': 'application/json' })
      res.end(formatPortFile(port, process.pid, identity))
    } else if (req.method === 'GET' && url.pathname === '/css') {
      const core = await preparation.wait()
      const { css, policy } = await core.css({
        fileNum: parseFileNum(url.searchParams.get('fileNum')),
        importMainCss: url.searchParams.get('importMainCss') === 'true',
        wait: url.searchParams.get('waitForIdle') === 'true',
      })
      res.writeHead(200, {
        'Content-Type': 'text/css',
        'x-devup-css-policy': policy,
      })
      res.end(css)
    } else if (req.method === 'POST' && url.pathname === '/extract') {
      let body: string
      reading.add(req)
      try {
        body = await readBody(req)
      } finally {
        reading.delete(req)
      }
      const request = parseExtractRequest(body)
      const core = await preparation.wait()
      sendJson(res, 200, await core.extract(request))
    } else {
      res.writeHead(404, { 'Content-Type': 'text/plain' })
      res.end('Not Found')
    }
  }

  async function handle(
    req: IncomingMessage,
    res: ServerResponse,
  ): Promise<void> {
    try {
      const url = new URL(req.url ?? '/', 'http://127.0.0.1')
      assertOwnership(req, url.pathname, identity, enforce)
      if (draining) {
        throw new HttpError(
          503,
          new CoordinatorShutdownError(url.pathname).message,
        )
      }
      await route(req, res, url)
    } catch (error) {
      sendJson(res, error instanceof HttpError ? error.status : 500, {
        error: error instanceof Error ? error.message : String(error),
        ...(error instanceof IncompleteCssError
          ? { missing: error.missing, failed: [...error.failed.keys()] }
          : {}),
      })
    }
  }

  const server = createServer((req, res) => {
    // `handle` answers every failure itself, so these promises never reject.
    const task = handle(req, res)
    inflight.add(task)
    void task.finally(() => inflight.delete(task))
  })
  server.on('connection', (socket) => {
    sockets.add(socket)
    socket.once('close', () => sockets.delete(socket))
  })

  function listen(): Promise<number> {
    return new Promise((resolveListen, reject) => {
      server.once('error', reject)
      server.listen(0, '127.0.0.1', () => {
        const address = server.address()
        resolveListen(
          typeof address === 'object' && address !== null ? address.port : 0,
        )
      })
    })
  }

  async function settle(): Promise<void> {
    while (inflight.size > 0) await Promise.all(inflight)
  }

  const closedFirst = Promise.withResolvers<undefined>()

  function close(): void {
    if (closed) return
    closed = true
    draining = true
    closedFirst.resolve(undefined)
    preparation.close()
    server.close()
    server.closeAllConnections()
    for (const socket of sockets) socket.destroy()
    removeOwnPortFile(portFile, identity)
  }

  const ready = (async () => {
    if (!('prepare' in options)) {
      preparation.start()
      await Promise.race([prepared, closedFirst.promise])
    }
    if (closed) return
    // A server closed while its socket opens may never report listening
    const listening = await Promise.race([listen(), closedFirst.promise])
    if (listening === undefined) {
      server.close()
      return
    }
    port = listening
    server.unref()
    publishPortFile(portFile, {
      version: 1,
      pid: process.pid,
      port,
      ...identity,
    })
    preparation.start()
  })()

  async function flush(): Promise<void> {
    try {
      await settlePreparation()
    } finally {
      do {
        await settle()
        await preparation.flush()
      } while (inflight.size > 0)
    }
  }

  async function settlePreparation(): Promise<void> {
    try {
      await prepared
    } catch (error) {
      if (!(error instanceof CoordinatorShutdownError)) throw error
    }
  }

  return {
    identity,
    ready,
    prepared,
    close,
    async drain() {
      draining = true
      preparation.stopWatching()
      if ('prepare' in options) preparation.close()
      for (const req of reading)
        req.destroy(new CoordinatorShutdownError('/extract'))
      try {
        await ready
        if (!('prepare' in options)) {
          await prepared
          preparation.close()
        }
        await settlePreparation()
      } finally {
        try {
          await settle()
          await preparation.flush()
        } finally {
          close()
        }
      }
    },
    flush,
  }
}
