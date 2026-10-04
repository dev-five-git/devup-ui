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
import { createCore } from './coordinator-core'
import {
  assertOwnership,
  HttpError,
  parseExtractRequest,
  parseFileNum,
  readBody,
  sendJson,
} from './coordinator-http'
import type { CoordinatorOptions } from './coordinator-options'
import {
  type CoordinatorIdentity,
  formatPortFile,
  publishPortFile,
  removeOwnPortFile,
} from './coordinator-port'
import { type SourceWatcher, watchSources } from './coordinator-watch'

export interface CoordinatorInstance {
  readonly identity: CoordinatorIdentity
  readonly ready: Promise<void>
  /** Stop listening and watching now. Accepted writes may still be running. */
  close(): void
  /** Refuse new requests, wait for accepted work and its writes, then close. */
  drain(): Promise<void>
  /** Wait for accepted work and its writes without closing. */
  flush(): Promise<void>
}

function reportBackgroundError(error: unknown): void {
  console.error(
    '[devup-ui]',
    error instanceof Error ? error.message : String(error),
  )
}

/** One app's coordinator: an HTTP endpoint over its own core and engine. */
export function createInstance(
  options: CoordinatorOptions,
): CoordinatorInstance {
  const root = resolve(options.projectRoot ?? process.cwd())
  const portFile = resolve(root, options.coordinatorPortFile)
  const identity = options.identity ?? { project: root, token: randomUUID() }
  const enforce = options.identity !== undefined
  const core = createCore(options, identity.project)
  const inflight = new Set<Promise<void>>()
  const sockets = new Set<Socket>()
  const reading = new Set<IncomingMessage>()
  let draining = false
  let closed = false
  let port = 0
  let watcher: SourceWatcher | undefined

  async function route(
    req: IncomingMessage,
    res: ServerResponse,
    url: URL,
  ): Promise<void> {
    if (req.method === 'GET' && url.pathname === '/health') {
      res.writeHead(200, { 'Content-Type': 'application/json' })
      res.end(formatPortFile(port, process.pid, identity))
    } else if (req.method === 'GET' && url.pathname === '/css') {
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
      sendJson(res, 200, await core.extract(parseExtractRequest(body)))
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
    watcher?.close()
    core.close()
    server.close()
    server.closeAllConnections()
    for (const socket of sockets) socket.destroy()
    removeOwnPortFile(portFile, identity)
  }

  const ready = (async () => {
    await core.startup()
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
    if (options.watch && !draining) {
      watcher = watchSources({
        roots: (options.sourceRoots ?? []).map((dir) => resolve(root, dir)),
        debounceMs: 50,
        onChange: () => void core.reconcile().catch(reportBackgroundError),
        onError: reportBackgroundError,
      })
    }
  })()

  async function flush(): Promise<void> {
    do {
      await settle()
      await core.flush()
    } while (inflight.size > 0)
  }

  return {
    identity,
    ready,
    close,
    async drain() {
      draining = true
      watcher?.close()
      core.close()
      for (const req of reading)
        req.destroy(new CoordinatorShutdownError('/extract'))
      try {
        await ready
        await flush()
      } finally {
        close()
      }
    },
    flush,
  }
}
